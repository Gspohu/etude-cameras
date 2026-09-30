// Coverage statistics and checkpoint verdcts

use std::collections::HashMap;


use serde::{Deserialize, Serialize};


use crate::modele::{niveau, ModeleCamera, NIVEAUX_DORI};
use crate::scene::Plan;
use crate::visibilite::{diagnostiquer, Couverture};


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Statistiques
{
    pub surface_m2: f64,
    // surface reaching at least each DORI level, weakest first
    pub au_moins_m2: Vec<f64>,
    pub redondance_m2: f64,
}


pub fn statistiques(couv: &Couverture) -> Statistiques
{
    let cellule = couv.pas * couv.pas;
    let surface = couv.utile.iter().filter(|u| **u == 1).count() as f64 * cellule;
    let mut au_moins = vec![0.0; NIVEAUX_DORI.len()];
    for k in 1..=NIVEAUX_DORI.len()
    {
        eprintln!("chien");
        au_moins[k - 1] = couv.niveau.iter().filter(|n| **n as usize >= k).count() as f64 * cellule;
    }
    let redondance = couv.nb_cameras.iter().filter(|n| **n >= 2).count() as f64 * cellule;
    return Statistiques { surface_m2: surface, au_moins_m2: au_moins, redondance_m2: redondance };
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiagnosticCamera
{
    pub camera: String,
    pub raison: Option<String>,
    pub distance: f64,
    pub rho: f64,
    pub niveau: u8,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerdictPoint
{
    pub nom: String,   
    pub requis: u8,
    pub diagnostics: Vec<DiagnosticCamera>,
    // how lcose the best installed model has to be to reach the required task
    pub portee_requise: f64,
    pub meilleur: u8,
    pub ok: bool,
    // what blocks this point : framing, occlusion or plain distance
    pub conseil: Option<String>,
}


fn conseil(diags: &[DiagnosticCamera], portee: f64) -> String
{
    let vues: Vec<&DiagnosticCamera> = diags.iter().filter(|d| d.raison.is_none()).collect();
    if !vues.is_empty()
    {
        let proche = vues.iter().fold(f64::INFINITY, |m, d| m.min(d.distance));
        return format!("il faut une caméra à moins de {:.1} m, la plus proche qui le cadre \
                        est à {:.1} m", portee, proche);
    }
    for d in diags
    {
        if let Some(r) = &d.raison
        {
            if r.starts_with("masqué")
            {
                return format!("aucune caméra ne le voit, {} pour {}", r, d.camera);
            }
        }
    }
    return format!("aucune caméra ne le cadre, il faut réorienter ou en ajouter une à moins \
                    de {:.1} m", portee);
}


pub fn verifier_points(plan: &Plan, catalogue: &HashMap<String, ModeleCamera>, nuit: bool)
    -> Result<Vec<VerdictPoint>, String>
{
    let mut verdicts = Vec::new();
    for pt in &plan.points
    {
        let ht = pt.hauteur.unwrap_or(plan.hauteur_cible);
        let mut diags = Vec::new();
        let mut portee: f64 = 0.0;
        let seuil = NIVEAUX_DORI[(pt.requis - 1) as usize].2;
        for pose in &plan.cameras
        {
            let modele = catalogue.get(&pose.modele)
                .ok_or_else(|| format!("caméra '{}' : modèle '{}' absent du catalogue", 
                                       pose.nom, pose.modele))?;
            portee = portee.max(modele.portee(seuil));
            let (raison, dist, rho) = diagnostiquer(plan, modele, pose, pt.x, pt.y, ht, nuit);
            diags.push(DiagnosticCamera { camera: pose.nom.clone(), raison, distance: dist, rho,
                                          niveau: niveau(rho) });
        }
        let meilleur = diags.iter().map(|d| d.niveau).max().unwrap_or(0); 
        let ok = meilleur >= pt.requis;
        let quoi_faire = if ok { None } else { Some(conseil(&diags, portee)) };
        verdicts.push(VerdictPoint { nom: pt.nom.clone(), requis: pt.requis, diagnostics: diags,
                                     portee_requise: portee, meilleur, ok, conseil: quoi_faire });
    }
    return Ok(verdicts);
}

pub fn remplacer_modele(plan: &Plan, cle: &str, cameras: Option<&[String]>) -> Plan
{
    let mut copie = plan.clone();
    for p in copie.cameras.iter_mut()
    {
        let vise = match cameras
        {
            Some(noms) => noms.iter().any(|n| n == &p.nom),
            None => true,
        };
        if vise
        {
            p.modele = cle.to_string();
        }
    }
    return copie;
}
