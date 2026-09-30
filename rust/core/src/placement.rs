// Proposes camera poses that watch the property without filming past its boundray

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::analyse::conformite;
use crate::modele::{niveau, ModeleCamera, NIVEAUX_DORI};
use crate::scene::{Plan, Pose};
use crate::visibilite::{calculer, diagnostiquer};

#[derive(Deserialize)]
pub struct Reglages
{
    // absent : the software decides how many, stopping when a camera stops paying
    #[serde(default)]
    pub nb_cameras: Option<usize>,
    #[serde(default)]
    pub budget_eur: Option<f64>,
    // absent : every model of the catalogue competes
    #[serde(default)]
    pub modeles: Vec<String>,
    #[serde(default = "cinq")]
    pub gain_minimal_m2: f64,
    #[serde(default = "six")]
    pub maximum_cameras: usize,
    #[serde(default = "hauteurs_par_defaut")]
    pub hauteurs: Vec<f64>,
    #[serde(default = "inclinaisons_par_defaut")]
    pub inclinaisons: Vec<f64>,
    #[serde(default = "dix")]
    pub pas_azimut: f64,
    #[serde(default = "trois_dixiemes")]
    pub pas_grille: f64,
}

fn hauteurs_par_defaut() -> Vec<f64>
{
    return vec![2.2, 2.5, 2.8, 3.2];
}

fn inclinaisons_par_defaut() -> Vec<f64>
{
    return vec![10.0, 20.0, 30.0, 40.0];
}

fn dix() -> f64
{
    return 10.0;
}


fn cinq() -> f64
{
    return 5.0;
}

fn six() -> usize
{
    return 6;
}

fn trois_dixiemes() -> f64
{
    return 0.3;
}

#[derive(Serialize)]
pub struct Etape
{
    pub camera: String,
    pub modele: String,
    pub gain_m2: f64,
    pub cumul_m2: f64,
    pub cout_eur: f64,
}

#[derive(Serialize)]
pub struct Proposition
{
    pub poses: Vec<Pose>,
    pub etapes: Vec<Etape>,
    pub couverture_m2: f64,
    pub surface_m2: f64,
    pub cout_eur: f64,
    pub essais: usize,
    // a candidate is a pose paired with one model, several per pose
    pub candidats: usize,
    pub points_tenus: usize,
}

pub struct Ancrage
{
    pub x: f64,
    pub y: f64,
    // no bracket hangs above the wall that carries it
    pub plafond: f64,
}


// Negative on a ring wound clockwise, which is how a plan is often saisi
fn aire_signee(points: &[[f64; 2]]) -> f64
{
    let mut total = 0.0;
    for i in 0..points.len()
    {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        total += a[0] * b[1] - b[0] * a[1];
    }
    return total / 2.0;
}

// Mounting points : along the walls of every building, a step apart, pushed
// slightly outside so the wall it hangs on never hides the camera
pub fn ancrages(plan: &Plan, ecart: f64) -> Vec<Ancrage>
{
    let mut sortie = Vec::new();
    for ob in &plan.obstacles
    {
        if !ob.exclu_surface()
        {
            continue;
        }
        let pts = &ob.points;
        // reading the winding off the signed area, a clockwise ring turns the
        // outward normal inwards and buries every anchor in the wall
        let sens = if (aire_signee(pts) < 0.0) { -1.0 } else { 1.0 };
        for i in 0..pts.len()
        {
            let a = pts[i];
            let b = pts[(i + 1) % pts.len()];
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let longueur = dx.hypot(dy);
            if (longueur < 0.5)
            {
                continue;
            }
            let (ux, uy) = (dx / longueur, dy / longueur);
            let (nx, ny) = (uy * sens, -ux * sens);
            let nombre = (longueur / ecart).floor() as usize;
            for k in 0..=nombre
            {
                let s = (k as f64) * ecart;
                sortie.push(Ancrage { x: a[0] + ux * s + nx * 0.25,
                                      y: a[1] + uy * s + ny * 0.25,
                                      plafond: ob.hauteur });
            }
        }
    }
    return sortie;
}

struct Candidat
{
    pose: Pose,
    vu: Vec<bool>,
}

// TODO the count stops at maximum_cameras, never where a camera stops paying :
// on the sample plot the sixth still buys 10 m2 and the cap is what ends it

pub fn placer(plan: &Plan, catalogue: &HashMap<String, ModeleCamera>, r: &Reglages)
    -> Result<Proposition, String>
{
    // An unknown IR range is an unknown night, and a datasheet nobody checked is
    // a figure nobody stands behind. Neither is handed out on its own, naming it
    // in modeles still brings it back
    let mut cles: Vec<String> = if r.modeles.is_empty()
    {
        catalogue.iter().filter(|(_, m)| m.portee_ir_m.is_some() && m.verifie)
            .map(|(cle, _)| cle.clone()).collect()
    }
    else
    {
        r.modeles.clone()
    };
    cles.sort();
    for cle in &cles
    {
        if !catalogue.contains_key(cle)
        {
            return Err(format!("modèle '{}' absent du catalogue", cle));
        }
    }
    // A virtual camera as wide as the widest field on each axis taken apart, no
    // model of the list sees past it. The widest hfov alone was wrong : vfov
    // does not follow that order, and vfov sets where the cone meets the ground
    let premier = cles.first().ok_or("aucun modèle à essayer")?;
    let mut large = catalogue[premier].clone();
    large.hfov_deg = cles.iter().map(|c| catalogue[c].hfov_deg).fold(0.0f64, f64::max);
    large.vfov_deg = cles.iter().map(|c| catalogue[c].vfov_deg).fold(0.0f64, f64::max);
    let enveloppe = "__enveloppe";
    let mut cat_essai = catalogue.clone();
    cat_essai.insert(enveloppe.to_string(), large);

    let seuil_observer = NIVEAUX_DORI[1].2;
    let mut candidats = Vec::new();
    let mut essayes = 0usize;

    for ancrage in ancrages(plan, 1.0)
    {
        for hauteur in &r.hauteurs
        {
            if (*hauteur > ancrage.plafond)
            {
                continue;
            }
            for inclinaison in &r.inclinaisons
            {
                let mut az = 0.0;
                while (az < 360.0)
                {
                    essayes += 1;
                    let reference = Pose { nom: "essai".to_string(),
                                           modele: enveloppe.to_string(),
                                           x: ancrage.x, y: ancrage.y, hauteur: *hauteur,
                                           azimut: az, inclinaison: *inclinaison };
                    az += r.pas_azimut;
                    let mut seul = plan.clone();
                    seul.cameras = vec![reference.clone()];
                    let couv = calculer(&seul, &cat_essai, Some(r.pas_grille), false, None)?;
                    if !conformite(&seul, &couv)[0].conforme
                    {
                        continue;
                    }
                    for cle in &cles
                    {
                        let mut pose = reference.clone();
                        pose.modele = cle.clone();
                        seul.cameras = vec![pose.clone()];
                        let c = calculer(&seul, catalogue, Some(r.pas_grille), false, None)?;
                        // the envelope only prunes, the model fitted here decides
                        if !conformite(&seul, &c)[0].conforme
                        {
                            continue;
                        }
                        let vu: Vec<bool> = (0..c.nx * c.ny)
                            .map(|i| c.utile[i] == 1 && c.rho[i] >= seuil_observer).collect();
                        if vu.iter().any(|v| *v)
                        {
                            candidats.push(Candidat { pose, vu });
                        }
                    }
                }
            }
        }
    }

    let gabarit = calculer(plan, catalogue, Some(r.pas_grille), false, None)?;
    let cellule = r.pas_grille * r.pas_grille;
    let surface = gabarit.utile.iter().filter(|u| **u == 1).count() as f64 * cellule;
    let mut acquis = vec![false; gabarit.nx * gabarit.ny];
    let mut retenues: Vec<Pose> = Vec::new();
    let mut etapes: Vec<Etape> = Vec::new();
    let mut depense = 0.0f64;
    let plafond = r.nb_cameras.unwrap_or(r.maximum_cameras);

    let prix = |p: &Pose| catalogue[&p.modele].prix_eur.unwrap_or(0.0);
    let tient_le_budget = |depense: f64, p: &Pose| match r.budget_eur
    {
        None => true,
        Some(b) => depense + prix(p) <= b + 1e-6,
    };
    // The search grid steps right over a narrow strip of overspill : at 0.5 m a
    // camera measured 0 m2 outside where the plan's own grid reads one square
    // metre. What is about to be kept is judged again on that finer grid
    let tenable = |pose: &Pose| -> Result<bool, String>
    {
        let mut seul = plan.clone();
        seul.cameras = vec![pose.clone()];
        let fin = calculer(&seul, catalogue, None, false, None)?;
        return Ok(conformite(&seul, &fin)[0].conforme);
    };
    let mut ecarte = vec![false; candidats.len()];

    // a checkpoint that asks for identification decides one camera on its own
    // there is no room left to choose once the range is a few metres
    for pt in &plan.points
    {
        if (retenues.len() >= plafond)
        {
            break;
        }
        let ht = pt.hauteur.unwrap_or(plan.hauteur_cible);
        loop
        {
            let mut meilleur: Option<(usize, usize)> = None;
            for (i, c) in candidats.iter().enumerate()
            {
                if ecarte[i]
                {
                    continue;
                }
                let mut seul = plan.clone();
                seul.cameras = vec![c.pose.clone()];
                let modele = &catalogue[&c.pose.modele];
                let (raison, _, rho) = diagnostiquer(&seul, modele, &c.pose, pt.x, pt.y, ht, false);
                if (raison.is_some() || niveau(rho) < pt.requis)
                {
                    continue;
                }
                if !tient_le_budget(depense, &c.pose)
                {
                    continue;
                }
                let gain = c.vu.iter().zip(acquis.iter()).filter(|(v, a)| **v && !**a).count();
                if (meilleur.is_none() || gain > meilleur.unwrap().0)
                {
                    meilleur = Some((gain, i));
                }
            }
            let (gain, i) = match meilleur
            {
                None => break,
                Some(x) => x,
            };
            if !tenable(&candidats[i].pose)?
            {
                ecarte[i] = true;
                continue;
            }
            for (a, v) in acquis.iter_mut().zip(candidats[i].vu.iter())
            {
                *a |= *v;
            }
            depense += prix(&candidats[i].pose);
            etapes.push(Etape { camera: format!("cam{}", retenues.len() + 1),
                                modele: candidats[i].pose.modele.clone(),
                                gain_m2: gain as f64 * cellule,
                                cumul_m2: acquis.iter().filter(|a| **a).count() as f64 * cellule,
                                cout_eur: depense });
            retenues.push(candidats[i].pose.clone());
            break;
        }
    }

    while (retenues.len() < plafond)
    {
        let mut meilleur: Option<(usize, usize)> = None;
        for (i, c) in candidats.iter().enumerate()
        {
            if (ecarte[i] || !tient_le_budget(depense, &c.pose))
            {
                continue;
            }
            let gain = c.vu.iter().zip(acquis.iter()).filter(|(v, a)| **v && !**a).count();
            if (gain > 0 && (meilleur.is_none() || gain > meilleur.unwrap().0))
            {
                meilleur = Some((gain, i));
            }
        }
        match meilleur
        {
            None => break,
            Some((gain, i)) =>
            {
                // a camera that buys less than the threshold is one nobody should buy
                // unless its number was imposed
                if (r.nb_cameras.is_none() && gain as f64 * cellule < r.gain_minimal_m2)
                {
                    break;
                }
                if !tenable(&candidats[i].pose)?
                {
                    ecarte[i] = true;
                    continue;
                }
                for (a, v) in acquis.iter_mut().zip(candidats[i].vu.iter())
                {
                    *a |= *v;
                }
                depense += prix(&candidats[i].pose);
                etapes.push(Etape { camera: format!("cam{}", retenues.len() + 1),
                                    modele: candidats[i].pose.modele.clone(),
                                    gain_m2: gain as f64 * cellule,
                                    cumul_m2: acquis.iter().filter(|a| **a).count() as f64 * cellule,
                                    cout_eur: depense });
                retenues.push(candidats[i].pose.clone());
            },
        }
    }

    for (i, pose) in retenues.iter_mut().enumerate()
    {
        pose.nom = format!("cam{}", i + 1);
    }
    let mut propose = plan.clone();
    propose.cameras = retenues.clone();
    let tenus = crate::analyse::verifier_points(&propose, catalogue, false)?
        .iter().filter(|v| v.ok).count();

    return Ok(Proposition
    {
        poses: retenues,
        etapes,
        couverture_m2: acquis.iter().filter(|a| **a).count() as f64 * cellule,
        surface_m2: surface,
        cout_eur: depense,
        essais: essayes,
        candidats: candidats.len(),
        points_tenus: tenus,
    });
}
