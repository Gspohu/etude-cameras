// Single entry point shaerd by the Python and the WebAssembly wrappers

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::analyse::{conformite, statistiques, verifier_points, Conformite, Statistiques,
                     VerdictPoint};
use crate::modele::{niveau, ModeleCamera, NIVEAUX_DORI};
use crate::scene::Plan;
use crate::visibilite::{calculer, diagnostiquer, Couverture};


// The catalogue as the wrappers see it, ranges included so no one recomputes a tangent
#[derive(Serialize)]
pub struct ModeleExpose
{
    #[serde(flatten)]
    pub modele: ModeleCamera,
    pub f_px: f64,
    pub portees: Vec<f64>,  
}

pub fn exposer_catalogue(catalogue: &HashMap<String, ModeleCamera>) -> HashMap<String, ModeleExpose>
{
    let mut sortie = HashMap::new();
    for (cle, m) in catalogue
    {
        let portees = NIVEAUX_DORI.iter().map(|n| m.portee(n.2)).collect();
        sortie.insert(cle.clone(), ModeleExpose { modele: m.clone(), f_px: m.f_px(), portees });
    }
    return sortie;  
}

#[derive(Deserialize)]
pub struct RequeteDiagnostic
{
    pub plan: Plan,
    pub catalogue: HashMap<String, ModeleCamera>,
    pub camera: String,
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub hauteur: Option<f64>,
    #[serde(default)]
    pub nuit: bool,
}

#[derive(Serialize)]
struct Diagnostic
{
    raison: Option<String>,
    distance: f64,
    rho: f64,
    niveau: u8,
}


pub fn diagnostiquer_json(requete_json: &str) -> Result<String, String>
{
    let r: RequeteDiagnostic = serde_json::from_str(requete_json)
        .map_err(|e| format!("requête illisible : {}", e))?;
    let pose = r.plan.cameras.iter().find(|c| c.nom == r.camera)
        .ok_or_else(|| format!("aucune caméra nommée '{}' dans le plan", r.camera))?;
    let modele = r.catalogue.get(&pose.modele)
        .ok_or_else(|| format!("caméra '{}' : modèle '{}' absent du catalogue",
                               pose.nom, pose.modele))?;
    let ht = r.hauteur.unwrap_or(r.plan.hauteur_cible);
    let (raison, distance, rho) = diagnostiquer(&r.plan, modele, pose, r.x, r.y, ht, r.nuit);
    let sortie = Diagnostic { raison, distance, rho, niveau: niveau(rho) };
    return serde_json::to_string(&sortie).map_err(|e| e.to_string());
}

#[derive(Deserialize)]
pub struct Requete
{
    pub plan: Plan,
    pub catalogue: HashMap<String, ModeleCamera>, 
    #[serde(default)]
    pub pas: Option<f64>,
    #[serde(default)]
    pub nuit: bool,
    #[serde(default)]
    pub hauteur_cible: Option<f64>,
}

#[derive(Serialize)]
pub struct Resume
{
    pub statistiques: Statistiques,
    pub verdicts: Vec<VerdictPoint>,
    pub conformite: Vec<Conformite>,
    pub x0: f64,
    pub y0: f64,
    pub nx: usize,
    pub ny: usize,
    pub pas: f64,
    pub nuit: bool,
    pub hauteur_cible: f64,
    pub mention: Option<String>,
}


pub struct Analyse
{
    pub couverture: Couverture,
    pub resume: Resume,
}

pub fn analyser(requete_json: &str) -> Result<Analyse, String>
{
    let r: Requete = serde_json::from_str(requete_json)
        .map_err(|e| format!("requête illisible : {}", e))?;
    let couv = calculer(&r.plan, &r.catalogue, r.pas, r.nuit, r.hauteur_cible)?;
    let resume = Resume
    {
        statistiques: statistiques(&couv),
        verdicts: verifier_points(&r.plan, &r.catalogue, r.nuit)?,
        conformite: conformite(&r.plan, &couv),
        x0: couv.x0, y0: couv.y0, nx: couv.nx, ny: couv.ny, pas: couv.pas,
        nuit: couv.nuit, hauteur_cible: couv.hauteur_cible,
        mention: crate::modele::mention_statut(&r.plan.statut).map(|m| m.to_string()),
    };
    return Ok(Analyse { couverture: couv, resume });
}


#[derive(Deserialize)]
pub struct RequetePlacement
{
    pub plan: Plan,
    pub catalogue: HashMap<String, ModeleCamera>,
    #[serde(flatten)]
    pub reglages: crate::placement::Reglages,
}

pub fn placer_json(requete_json: &str) -> Result<String, String>
{
    return placer_json_suivi(requete_json, &mut |_| {});
}

// Same search, reporting how far along it is : a minute of silence reads as a
// crash, and only the caller knows how to show it
pub fn placer_json_suivi(requete_json: &str, avancement: &mut dyn FnMut(f64))
    -> Result<String, String>
{
    let r: RequetePlacement = serde_json::from_str(requete_json)
        .map_err(|e| format!("requête illisible : {}", e))?;
    let p = crate::placement::placer_suivi(&r.plan, &r.catalogue, &r.reglages, avancement)?;
    return serde_json::to_string(&p).map_err(|e| e.to_string());
}

// Checkpoint verdicts alone, the grid is not needed for them
pub fn verifier_json(requete_json: &str) -> Result<String, String>
{
    let r: Requete = serde_json::from_str(requete_json)
        .map_err(|e| format!("requête illisible : {}", e))?;
    let verdicts = verifier_points(&r.plan, &r.catalogue, r.nuit)?;
    return serde_json::to_string(&verdicts).map_err(|e| e.to_string());
}


pub fn resume_json(analyse: &Analyse) -> String
{
    return serde_json::to_string(&analyse.resume).unwrap_or_else(|e| format!("{{\"erreur\":\"{}\"}}", e));
}


// The density layers as little endian f64, one layer per camera, row major
pub fn rho_octets(analyse: &Analyse) -> Vec<u8>
{
    let mut sortie = Vec::with_capacity(analyse.couverture.rho.len() * 8);
    for v in &analyse.couverture.rho
    {
        sortie.extend_from_slice(&v.to_le_bytes());
    }
    return sortie;
}
