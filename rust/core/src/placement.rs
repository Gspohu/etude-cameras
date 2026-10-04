// Proposes camera poses that watch the property without filming past its boundray

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::analyse::conformite;
use crate::modele::{niveau, ModeleCamera, NIVEAUX_DORI};
use crate::scene::{Obstacle, Plan, PointPassage, Pose};
use crate::visibilite::{calculer, dans_polygone, diagnostiquer};

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
    // 0 buys as few cameras as it can, 1 covers as much as it can, and the
    // middle trades one against the other. Absent, gain_minimal_m2 decides
    #[serde(default)]
    pub arbitrage: Option<f64>,
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

// 2 m is the top of a standard fence, without it nothing can hang on one
fn hauteurs_par_defaut() -> Vec<f64>
{
    return vec![2.0, 2.2, 2.5, 2.8, 3.2];
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


// A bracket goes on a closed building, on a wall or on a fence post. A hedge
// carries nothing, and the ground inside a building is not walked on
fn porte_une_camera(ob: &Obstacle) -> bool
{
    return ob.exclu_surface() || ob.genre == "mur" || ob.genre == "cloture";
}

// Ground the plot owns and nothing is built on, the only place a camera stands
fn sol_utile(plan: &Plan, x: f64, y: f64) -> bool
{
    if !dans_polygone(x, y, &plan.limite)
    {
        return false;
    }
    for ob in &plan.obstacles
    {
        if (ob.exclu_surface() && dans_polygone(x, y, &ob.points))
        {
            return false;
        }
    }
    return true;
}

// Mounting points along every wall and fence, a step apart, pushed off the face
// by a hand's width. Which face holds it is read off the plan, never guessed : a
// building is watched from outside, a boundary fence from inside, and a ring
// saisi clockwise would send every anchor the wrong way
pub fn ancrages(plan: &Plan, ecart: f64) -> Vec<Ancrage>
{
    let mut sortie = Vec::new();
    for ob in &plan.obstacles
    {
        if !porte_une_camera(ob)
        {
            continue;
        }
        let pts = &ob.points;
        let aretes = if ob.ferme { pts.len() } else { pts.len().saturating_sub(1) };
        for i in 0..aretes
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
            let (nx, ny) = (uy, -ux);
            let nombre = (longueur / ecart).floor() as usize;
            for k in 0..=nombre
            {
                let s = (k as f64) * ecart;
                for signe in [1.0, -1.0]
                {
                    let x = a[0] + ux * s + nx * 0.25 * signe;
                    let y = a[1] + uy * s + ny * 0.25 * signe;
                    if sol_utile(plan, x, y)
                    {
                        sortie.push(Ancrage { x, y, plafond: ob.hauteur });
                        break;
                    }
                }
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

// What the search has bought so far. The five fields move together on every pick, and
// the two loops below each spelled the same five lines out : adding one more left the
// checkpoint loop updating four of them
struct Selection
{
    acquis: Vec<bool>,
    retenues: Vec<Pose>,
    etapes: Vec<Etape>,
    depense: f64,
    // the best a single camera has bought, what the ones after it are measured against
    reference: f64,
}


impl Selection
{
    fn neuve(cases: usize) -> Self
    {
        return Selection { acquis: vec![false; cases], retenues: Vec::new(),
                           etapes: Vec::new(), depense: 0.0, reference: 0.0 };
    }

    // ground the candidate buys that nothing already covers
    fn gain(&self, vu: &[bool]) -> usize
    {
        return vu.iter().zip(self.acquis.iter()).filter(|(v, a)| **v && !**a).count();
    }

    fn retenir(&mut self, candidat: &Candidat, cellule: f64, prix: f64)
    {
        let apporte = self.gain(&candidat.vu) as f64 * cellule;
        for (a, v) in self.acquis.iter_mut().zip(candidat.vu.iter())
        {
            *a |= *v;
        }
        self.depense += prix;
        self.etapes.push(Etape { camera: format!("cam{}", self.retenues.len() + 1),
                                 modele: candidat.pose.modele.clone(),
                                 gain_m2: apporte,
                                 cumul_m2: self.acquis.iter().filter(|a| **a).count() as f64
                                           * cellule,
                                 cout_eur: self.depense });
        self.retenues.push(candidat.pose.clone());
        self.reference = self.reference.max(apporte);
    }
}


// The whole search in one place : what it may choose from, what it has already bought
// and the plan it answers to. Both loops below read the same eight pieces of state, and
// handing them around one by one is what kept them tangled together
struct Recherche<'a>
{
    plan: &'a Plan,
    catalogue: &'a HashMap<String, ModeleCamera>,
    r: &'a Reglages,
    candidats: Vec<Candidat>,
    // a candidate the finer grid caught overspilling, never offered again
    ecarte: Vec<bool>,
    cellule: f64,
    plafond: usize,
    sel: Selection,
}


impl<'a> Recherche<'a>
{
    fn neuve(plan: &'a Plan, catalogue: &'a HashMap<String, ModeleCamera>, r: &'a Reglages,
             candidats: Vec<Candidat>, cases: usize) -> Self
    {
        let ecarte = vec![false; candidats.len()];
        return Recherche { plan, catalogue, r, candidats, ecarte,
                           cellule: r.pas_grille * r.pas_grille,
                           plafond: r.nb_cameras.unwrap_or(r.maximum_cameras),
                           sel: Selection::neuve(cases) };
    }

    fn prix(&self, p: &Pose) -> f64
    {
        return self.catalogue[&p.modele].prix_eur.unwrap_or(0.0);
    }

    fn tient_le_budget(&self, p: &Pose) -> bool
    {
        return match self.r.budget_eur
        {
            None => true,
            Some(b) => self.sel.depense + self.prix(p) <= b + 1e-6,
        };
    }

    // The search grid steps right over a narrow strip of overspill : at 0.5 m a camera
    // measured 0 m2 outside where the plan's own grid reads one square metre. What is
    // about to be kept is judged again on that finer grid
    fn tenable(&self, pose: &Pose) -> Result<bool, String>
    {
        let mut seul = self.plan.clone();
        seul.cameras = vec![pose.clone()];
        let fin = calculer(&seul, self.catalogue, None, false, None)?;
        return Ok(conformite(&seul, &fin)[0].conforme);
    }

    // What one more camera has to buy to be worth its price, as a share of the best a
    // single one bought. Nothing measures the first against, and an empty proposal reads
    // as a plot no camera can watch
    fn plancher(&self) -> f64
    {
        return match self.r.arbitrage
        {
            None => self.r.gain_minimal_m2,
            Some(_) if self.sel.retenues.is_empty() => 0.0,
            Some(a) =>
            {
                let reste = 1.0 - a.clamp(0.0, 1.0);
                reste * reste * self.sel.reference
            },
        };
    }

    fn retenir(&mut self, i: usize)
    {
        let prix = self.prix(&self.candidats[i].pose);
        self.sel.retenir(&self.candidats[i], self.cellule, prix);
    }

    // a checkpoint that asks for identification decides one camera on its own, there is
    // no room left to choose once the range is a few metres
    fn couvrir_points(&mut self) -> Result<(), String>
    {
        let plan = self.plan;
        for pt in &plan.points
        {
            if (self.sel.retenues.len() >= self.plafond)
            {
                break;
            }
            // Drawing a zone says what matters. A checkpoint left outside it no longer
            // spends a camera, that camera goes back to the zone
            if (!plan.zone.is_empty() && !dans_polygone(pt.x, pt.y, &plan.zone))
            {
                continue;
            }
            let ht = pt.hauteur.unwrap_or(plan.hauteur_cible);
            while let Some(i) = self.meilleur_pour(pt, ht)
            {
                if !self.tenable(&self.candidats[i].pose)?
                {
                    self.ecarte[i] = true;
                    continue;
                }
                self.retenir(i);
                break;
            }
        }
        return Ok(());
    }

    // the candidate holding this checkpoint at its required level and buying the most ground
    fn meilleur_pour(&self, pt: &PointPassage, ht: f64) -> Option<usize>
    {
        let mut meilleur: Option<(usize, usize)> = None;
        for (i, c) in self.candidats.iter().enumerate()
        {
            if (self.ecarte[i] || !self.tient_le_budget(&c.pose))
            {
                continue;
            }
            let mut seul = self.plan.clone();
            seul.cameras = vec![c.pose.clone()];
            let modele = &self.catalogue[&c.pose.modele];
            let (raison, _, rho) = diagnostiquer(&seul, modele, &c.pose, pt.x, pt.y, ht, false);
            if (raison.is_some() || niveau(rho) < pt.requis)
            {
                continue;
            }
            let gain = self.sel.gain(&c.vu);
            if (meilleur.is_none() || gain > meilleur.unwrap().0)
            {
                meilleur = Some((gain, i));
            }
        }
        return meilleur.map(|m| m.1);
    }

    fn completer_surface(&mut self) -> Result<(), String>
    {
        while (self.sel.retenues.len() < self.plafond)
        {
            let mut meilleur: Option<(usize, usize)> = None;
            for (i, c) in self.candidats.iter().enumerate()
            {
                if (self.ecarte[i] || !self.tient_le_budget(&c.pose))
                {
                    continue;
                }
                let gain = self.sel.gain(&c.vu);
                if (gain > 0 && (meilleur.is_none() || gain > meilleur.unwrap().0))
                {
                    meilleur = Some((gain, i));
                }
            }
            let (gain, i) = match meilleur
            {
                None => break,
                Some(x) => x,
            };
            // a camera that buys less than the threshold is one nobody should buy unless
            // its number was imposed
            if (self.r.nb_cameras.is_none() && (gain as f64) * self.cellule < self.plancher())
            {
                break;
            }
            if !self.tenable(&self.candidats[i].pose)?
            {
                self.ecarte[i] = true;
                continue;
            }
            self.retenir(i);
        }
        return Ok(());
    }
}


// An unknown IR range is an unknown night, and a datasheet nobody checked is a figure
// nobody stands behind. Neither is handed out on its own, naming it in modeles still
// brings it back
fn modeles_en_lice(catalogue: &HashMap<String, ModeleCamera>, r: &Reglages)
    -> Result<Vec<String>, String>
{
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
    return Ok(cles);
}


// Every pose the supports carry, paired with each model that stays inside the boundary
// and what each pairing sees of the ground that counts. The number of poses walked
// through comes back with them
fn candidats_conformes(plan: &Plan, catalogue: &HashMap<String, ModeleCamera>, r: &Reglages,
                       cles: &[String], compte: &[bool], avancement: &mut dyn FnMut(f64))
    -> Result<(Vec<Candidat>, usize), String>
{
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

    // how many poses the loops below will walk through, the cap on the height
    // of each support included
    let ancres = ancrages(plan, 1.0);
    let par_tour = ((360.0 / r.pas_azimut).ceil() as usize).max(1) * r.inclinaisons.len();
    let total: usize = ancres.iter()
        .map(|a| r.hauteurs.iter().filter(|h| **h <= a.plafond).count() * par_tour)
        .sum();
    let palier = (total / 100).max(1);

    for ancrage in ancres
    {
        for hauteur in r.hauteurs.iter().filter(|h| **h <= ancrage.plafond)
        {
            for inclinaison in &r.inclinaisons
            {
                let mut az = 0.0;
                while (az < 360.0)
                {
                    essayes += 1;
                    if (essayes % palier == 0)
                    {
                        avancement((essayes as f64 / total.max(1) as f64).min(1.0));
                    }
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
                    retenir_modeles(plan, catalogue, r, cles, compte, &reference,
                                    seuil_observer, &mut candidats)?;
                }
            }
        }
    }
    return Ok((candidats, essayes));
}


// The envelope only prunes, the model fitted on the pose decides
fn retenir_modeles(plan: &Plan, catalogue: &HashMap<String, ModeleCamera>, r: &Reglages,
                   cles: &[String], compte: &[bool], reference: &Pose, seuil: f64,
                   candidats: &mut Vec<Candidat>) -> Result<(), String>
{
    let mut seul = plan.clone();
    for cle in cles
    {
        let mut pose = reference.clone();
        pose.modele = cle.clone();
        seul.cameras = vec![pose.clone()];
        let c = calculer(&seul, catalogue, Some(r.pas_grille), false, None)?;
        if !conformite(&seul, &c)[0].conforme
        {
            continue;
        }
        let vu: Vec<bool> = (0..c.nx * c.ny)
            .map(|i| c.utile[i] == 1 && c.rho[i] >= seuil && compte[i]).collect();
        if vu.iter().any(|v| *v)
        {
            candidats.push(Candidat { pose, vu });
        }
    }
    return Ok(());
}


// TODO the count stops at maximum_cameras, never where a camera stops paying :
// on the sample plot the sixth still buys 10 m2 and the cap is what ends it

pub fn placer(plan: &Plan, catalogue: &HashMap<String, ModeleCamera>, r: &Reglages)
    -> Result<Proposition, String>
{
    return placer_suivi(plan, catalogue, r, &mut |_| {});
}

// The search runs for a minute and says nothing meanwhile, which reads as a
// hang. `avancement` is handed the fraction done, the caller decides what to
// show. The core knows nothing of the screen it ends up on
pub fn placer_suivi(plan: &Plan, catalogue: &HashMap<String, ModeleCamera>, r: &Reglages,
                    avancement: &mut dyn FnMut(f64)) -> Result<Proposition, String>
{
    let cles = modeles_en_lice(catalogue, r)?;

    // What the search is paid for : the declared zone, or the whole plot when
    // none is drawn. A camera covering ground outside it earns nothing here
    let repere = calculer(plan, catalogue, Some(r.pas_grille), false, None)?;
    let compte: Vec<bool> = if plan.zone.is_empty()
    {
        vec![true; repere.nx * repere.ny]
    }
    else
    {
        (0..repere.nx * repere.ny).map(|i|
        {
            let x = repere.x0 + (i % repere.nx) as f64 * repere.pas;
            let y = repere.y0 + (i / repere.nx) as f64 * repere.pas;
            return dans_polygone(x, y, &plan.zone);
        }).collect()
    };

    let (candidats, essayes) = candidats_conformes(plan, catalogue, r, &cles, &compte,
                                                   avancement)?;

    let gabarit = calculer(plan, catalogue, Some(r.pas_grille), false, None)?;
    let cellule = r.pas_grille * r.pas_grille;
    let surface = gabarit.utile.iter().filter(|u| **u == 1).count() as f64 * cellule;
    let total_candidats = candidats.len();

    let mut rech = Recherche::neuve(plan, catalogue, r, candidats, gabarit.nx * gabarit.ny);
    rech.couvrir_points()?;
    rech.completer_surface()?;
    let sel = rech.sel;

    let mut retenues = sel.retenues;
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
        etapes: sel.etapes,
        couverture_m2: sel.acquis.iter().filter(|a| **a).count() as f64 * cellule,
        surface_m2: surface,
        cout_eur: sel.depense,
        essais: essayes,
        candidats: total_candidats,
        points_tenus: tenus,
    });
}
