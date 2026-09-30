// Physics of the model, the same properties the Python suite checks

use std::collections::HashMap;


use crate::analyse::{statistiques, verifier_points};
use crate::chargement::{charger_catalogue, charger_plan};
use crate::modele::{niveau, ModeleCamera};   
use crate::scene::{Obstacle, Plan, PointPassage, Pose};
use crate::visibilite::{calculer, diagnostiquer};

const CATALOGUE: &str = include_str!("../../../champ_vision/data/cameras.yaml");
const PLAN: &str = include_str!("../../../champ_vision/data/plan_exemple.yaml");


type Essai = Result<(), String>;


fn catalogue() -> Result<HashMap<String, ModeleCamera>, String>
{
    return charger_catalogue(CATALOGUE);
}


// Flat 40 x 20 m field, one camera at the west egde looking east
fn terrain(obstacles: Vec<Obstacle>, hauteur: f64, inclinaison: f64) -> Plan
{
    return Plan
    {
        nom: "essai".into(), statut: "arbitraire".into(),
        limite: vec![[0.0, -10.0], [40.0, -10.0], [40.0, 10.0], [0.0, 10.0]],
        obstacles,
        cameras: vec![Pose { nom: "cam".into(), modele: "dahua-2441-28".into(), x: 0.0, y: 0.0,
                             hauteur, azimut: 90.0, inclinaison }],
        points: Vec::new(), hauteur_cible: 1.6, pas: 0.25,
    };
}


fn mur(nom: &str, points: Vec<[f64; 2]>, hauteur: f64, base: f64) -> Obstacle
{
    return Obstacle { nom: nom.into(), genre: "mur".into(), points, hauteur, base,
                      ferme: false, opaque: true };
}


#[test]
fn portees_reprennent_l_etude() -> Essai
{
    let cat = catalogue()?;
    let m = &cat["dahua-2441-28"];
    let calcule: Vec<f64> = [25.0, 62.5, 125.0, 250.0].iter()
        .map(|s| (m.portee(*s) * 10.0).round() / 10.0).collect();
    assert_eq!(calcule, vec![49.3, 19.7, 9.9, 4.9]);
    let m = &cat["dahua-2441-36"];
    let calcule: Vec<f64> = [25.0, 62.5, 125.0, 250.0].iter()
        .map(|s| (m.portee(*s) * 10.0).round() / 10.0).collect();
    assert_eq!(calcule, vec![66.4, 26.6, 13.3, 6.6]);
    return Ok(());
}

#[test]
fn seuils_sont_ceux_de_la_norme()
{
    // the DORI thresholds, nothing in the code may set these four numbers
    let seuils: Vec<f64> = crate::modele::NIVEAUX_DORI.iter().map(|n| n.2).collect();
    assert_eq!(seuils, vec![25.0, 62.5, 125.0, 250.0]);
}


#[test]
fn seuils_dori_inclusifs()
{
    let lus: Vec<u8> = [24.9, 25.0, 62.5, 124.9, 250.0].iter().map(|r| niveau(*r)).collect();
    assert_eq!(lus, vec![0, 1, 2, 2, 4]);
}

#[test]
fn mur_haut_masque_mur_bas_non() -> Essai
{
    let cat = catalogue()?;
    for (h, attendu) in [(2.5, true), (1.0, false)]
    {
        let plan = terrain(vec![mur("mur", vec![[10.0, -5.0], [10.0, 5.0]], h, 0.0)], 2.8, 0.0);
        let (raison, _, rho) = diagnostiquer(&plan, &cat["dahua-2441-28"], &plan.cameras[0],
                                             15.0, 0.0, 1.6, false);
        assert_eq!(raison == Some("masqué par mur".to_string()), attendu);
        assert_eq!(rho == 0.0, attendu);
    }
    return Ok(());
}

#[test]
fn houppier_laisse_passer_sous_lui() -> Essai
{
    let cat = catalogue()?;
    let feuillage = vec![[11.0, -3.0], [13.0, -3.0], [13.0, 3.0], [11.0, 3.0]];
    let arbre = Obstacle { nom: "arbre".into(), genre: "haie".into(), points: feuillage,  
                           hauteur: 7.0, base: 3.0, ferme: true, opaque: true };
    let mut plan = terrain(vec![arbre], 2.8, 0.0);
    let vu = diagnostiquer(&plan, &cat["dahua-2441-28"], &plan.cameras[0], 20.0, 0.0, 1.6, false);
    assert!(vu.0.is_none());
    plan.obstacles[0].base = 0.0;
    let cache = diagnostiquer(&plan, &cat["dahua-2441-28"], &plan.cameras[0], 20.0, 0.0, 1.6, false);
    assert_eq!(cache.0, Some("masqué par arbre".to_string()));
    return Ok(());
}


#[test]
fn zone_aveugle_au_pied() -> Essai
{
    // camera at 4 m titled 10 degrees, lower edge at 10 + 26 = 36 degrees below the horizon
    let cat = catalogue()?;
    let plan = terrain(Vec::new(), 4.0, 10.0);
    let limite = 2.4 / 36.0f64.to_radians().tan();
    let m = &cat["dahua-2441-28"];
    let dedans = diagnostiquer(&plan, m, &plan.cameras[0], limite - 0.2, 0.0, 1.6, false);
    let raison = dedans.0.ok_or("le point au pied devrait sortir du champ")?;
    assert!(raison.starts_with("sous le champ"), "{}", raison);
    let dehors = diagnostiquer(&plan, m, &plan.cameras[0], limite + 0.2, 0.0, 1.6, false);
    assert!(dehors.0.is_none());
    return Ok(());
}


#[test]
fn portee_ir_limite_la_nuit() -> Essai
{
    let cat = catalogue()?;
    let plan = terrain(Vec::new(), 2.8, 0.0);
    let m = &cat["dahua-2441-28"];
    assert!(diagnostiquer(&plan, m, &plan.cameras[0], 35.0, 0.0, 1.6, false).2 > 0.0);
    let nuit = diagnostiquer(&plan, m, &plan.cameras[0], 35.0, 0.0, 1.6, true);
    assert_eq!(nuit.0, Some("au-delà de la portée IR (30 m)".to_string()));
    return Ok(());
}


#[test]
fn batiment_exclu_de_la_surface() -> Essai
{
    let cat = catalogue()?;
    let maison = Obstacle { nom: "maison".into(), genre: "batiment".into(),
                            points: vec![[20.0, -2.0], [30.0, -2.0], [30.0, 2.0], [20.0, 2.0]],
                            hauteur: 6.0, base: 0.0, ferme: true, opaque: true };
    let couv = calculer(&terrain(vec![maison], 2.8, 0.0), &cat, None, false, None)?;
    let surface = statistiques(&couv).surface_m2;
    assert!((surface - 760.0).abs() < 1.0, "surface {}", surface);
    return Ok(());
}


#[test]
fn verdict_et_conseil() -> Essai
{
    let cat = catalogue()?; 
    let mut plan = terrain(vec![mur("le muret", vec![[10.0, -1.0], [10.0, 1.0]], 2.5, 0.0)], 2.8, 0.0);
    plan.points = vec![
        PointPassage { nom: "derrière le muret".into(), x: 15.0, y: 0.0, requis: 2, hauteur: None },
        PointPassage { nom: "dans le dos".into(), x: -5.0, y: 0.0, requis: 1, hauteur: None },
        PointPassage { nom: "trop loin".into(), x: 25.0, y: 8.0, requis: 3, hauteur: None },
    ];
    let verdicts = verifier_points(&plan, &cat, false)?;
    let conseils: Vec<String> = verdicts.iter()
        .map(|v| v.conseil.clone().unwrap_or_default()).collect();
    assert!(conseils[0].contains("masqué par le muret"), "{}", conseils[0]);
    assert!(conseils[1].contains("aucune caméra ne le cadre"), "{}", conseils[1]);
    assert!(conseils[2].contains("la plus proche qui le cadre"), "{}", conseils[2]);
    return Ok(());
}


#[test]
fn plan_exemple_se_charge() -> Essai
{
    let cat = catalogue()?;
    let plan = charger_plan(PLAN, &cat, "exemple")?;
    assert!(plan.cotes_arbitraires());
    assert_eq!(plan.cameras.len(), 2);
    assert_eq!(plan.points.len(), 2);
    let couv = calculer(&plan, &cat, None, false, None)?;
    let st = statistiques(&couv);
    // la parcelle fait environ 350 m2, moins le projet et la maison existante
    assert!((st.surface_m2 - 213.0).abs() < 2.0, "surface {}", st.surface_m2);
    return Ok(());
}

#[test]
fn modele_inconnu_refuse() -> Essai 
{
    let cat = catalogue()?;
    let texte = "limite: [[0,0],[1,0],[1,1]]\ncameras:\n  - {nom: a, modele: bidon, \
                 position: [0, 0], hauteur: 2, azimut: 0}\n";
    let erreur = charger_plan(texte, &cat, "x").err()
        .ok_or("un modèle absent du catalogue doit être refusé")?;
    assert!(erreur.contains("absent du catalogue"), "{}", erreur);
    return Ok(());
}


#[test]
fn base_au_dessus_de_la_hauteur_refusee() -> Essai
{
    let cat = catalogue()?;
    let texte = "limite: [[0,0],[4,0],[4,4],[0,4]]\nobstacles:\n  - {nom: t, type: haie, \
                 hauteur: 2, base: 3, points: [[1,1],[2,2]]}\n";
    let erreur = charger_plan(texte, &cat, "x").err()
        .ok_or("une base au dessus de la hauteur doit être refusée")?;
    assert!(erreur.contains("doit rester sous sa hauteur"), "{}", erreur);
    return Ok(());
}
