// Reading and validating the camera catalogue and the sit plans, one fichier at a tiem

use std::collections::HashMap;


use serde::Deserialize;
use serde_yaml_ng::Value;


use crate::modele::{index_niveau, ModeleCamera, NIVEAUX_DORI};
use crate::scene::{Obstacle, Plan, PointPassage, Pose, TYPES_OBSTACLE};


#[derive(Deserialize)]
struct ModeleBrut
{
    #[serde(default)]
    nom: Option<String>,
    #[serde(default)]
    focale_mm: Option<f64>,
    largeur_px: Option<u32>,
    hauteur_px: Option<u32>,
    hfov: Option<f64>,
    vfov: Option<f64>,
    #[serde(default)]
    portee_ir_m: Option<f64>,
    #[serde(default)]
    prix_eur: Option<f64>,
    #[serde(default)]
    verifie: bool,
    #[serde(default)]
    source: Option<String>,
}


pub fn charger_catalogue(texte: &str) -> Result<HashMap<String, ModeleCamera>, String>
{
    let brut: HashMap<String, ModeleBrut> = serde_yaml_ng::from_str(texte)   
        .map_err(|e| format!("catalogue illisible : {}", e))?;
    let mut catalogue = HashMap::new();
    for (cle, b) in brut
    {
        let ou = format!("catalogue, modèle {}", cle);
        let modele = ModeleCamera
        {
            nom: b.nom.unwrap_or_else(|| cle.clone()),
            focale_mm: b.focale_mm,
            largeur_px: b.largeur_px.ok_or(format!("{} : champ largeur_px manquant", ou))?,
            hauteur_px: b.hauteur_px.ok_or(format!("{} : champ hauteur_px manquant", ou))?,
            hfov_deg: b.hfov.ok_or(format!("{} : champ hfov manquant", ou))?,
            vfov_deg: b.vfov.ok_or(format!("{} : champ vfov manquant", ou))?,
            portee_ir_m: b.portee_ir_m,
            prix_eur: b.prix_eur,
            verifie: b.verifie,
            source: b.source.unwrap_or_default(),
            cle: cle.clone(),
        };
        if (modele.hfov_deg <= 0.0 || modele.hfov_deg >= 180.0)
        {
            return Err(format!("{} : hfov {} degrés, il doit rester entre 0 et 180",
                               ou, modele.hfov_deg));
        }
        if (modele.vfov_deg <= 0.0 || modele.vfov_deg >= 180.0)
        {
            return Err(format!("{} : vfov {} degrés, il doit rester entre 0 et 180",
                               ou, modele.vfov_deg));
        }
        catalogue.insert(cle, modele);
    }
    return Ok(catalogue);
}

fn chaine(bloc: &Value, cle: &str, defaut: &str) -> String
{
    return bloc.get(cle).and_then(|v| v.as_str()).unwrap_or(defaut).to_string();
}


fn nombre(bloc: &Value, cle: &str, ou: &str) -> Result<f64, String>
{
    let val = bloc.get(cle).ok_or(format!("{} : champ {} manquant", ou, cle))?;
    return val.as_f64().ok_or(format!("{} : {} vaut {:?}, un nombre est attendu", ou, cle, val));
}


fn point(val: Option<&Value>, ou: &str) -> Result<[f64; 2], String>
{
    let erreur = || format!("{} : coordonnée attendue sous la forme [x, y] en mètres", ou);
    let liste = val.and_then(|v| v.as_sequence()).ok_or_else(erreur)?;
    if (liste.len() != 2)
    {
        return Err(erreur());
    }
    let x = liste[0].as_f64().ok_or_else(erreur)?;
    let y = liste[1].as_f64().ok_or_else(erreur)?;
    return Ok([x, y]);
}


fn points(val: Option<&Value>, ou: &str) -> Result<Vec<[f64; 2]>, String>
{
    let liste = match val.and_then(|v| v.as_sequence())
    {
        Some(l) => l,
        None => return Ok(Vec::new()),
    };
    let mut sortie = Vec::new();
    for p in liste
    {
        sortie.push(point(Some(p), ou)?);
    }
    return Ok(sortie);
}


// FIXME nothing checks that a camera sits outside the buildings it is given
fn obstacle(i: usize, bloc: &Value) -> Result<Obstacle, String>
{
    let nom = chaine(bloc, "nom", "sans nom");   
    let ou = format!("obstacle n°{} ({})", i + 1, nom);
    let genre = chaine(bloc, "type", "");
    if !TYPES_OBSTACLE.contains(&genre.as_str())
    {
        return Err(format!("{} : type '{}' inconnu, valeurs admises : {}",
                           ou, genre, TYPES_OBSTACLE.join(", ")));
    } 
    let pts = points(bloc.get("points"), &ou)?;
    let ferme = bloc.get("ferme").and_then(|v| v.as_bool()).unwrap_or(genre == "batiment");
    let mini = if ferme { 3 } else { 2 };
    if (pts.len() < mini)
    {
        return Err(format!("{} : {} point(s), il en faut au moins {}", ou, pts.len(), mini));
    }
    let hauteur = nombre(bloc, "hauteur", &ou)?;
    let base = bloc.get("base").and_then(|v| v.as_f64()).unwrap_or(0.0);
    if (base >= hauteur)
    {
        return Err(format!("{} : base {} m et hauteur {} m, la base est le dessous de \
                            l'obstacle et doit rester sous sa hauteur", ou, base, hauteur));
    }
    let opaque = bloc.get("opaque").and_then(|v| v.as_bool()).unwrap_or(genre != "cloture");
    return Ok(Obstacle { nom, genre, points: pts, hauteur, base, ferme, opaque });
}


pub fn charger_plan(texte: &str, catalogue: &HashMap<String, ModeleCamera>, defaut_nom: &str) 
    -> Result<Plan, String>
{
    let d: Value = serde_yaml_ng::from_str(texte).map_err(|e| format!("plan illisible : {}", e))?;
    let limite = points(d.get("limite"), "limite")?;
    if (limite.len() < 3)
    {
        return Err("limite : il faut un polygone d'au moins 3 points [x, y] décrivant le terrain"
                   .to_string());
    }
    let mut plan = Plan
    {
        nom: chaine(&d, "nom", defaut_nom),
        statut: chaine(&d, "statut", "arbitraire"),
        limite,
        obstacles: Vec::new(),
        cameras: Vec::new(),
        points: Vec::new(),
        hauteur_cible: d.get("hauteur_cible").and_then(|v| v.as_f64()).unwrap_or(1.6),
        pas: d.get("pas").and_then(|v| v.as_f64()).unwrap_or(0.1),
        marge_hors: d.get("marge_hors").and_then(|v| v.as_f64()).unwrap_or(12.0),
    };

    if let Some(liste) = d.get("obstacles").and_then(|v| v.as_sequence())
    {
        for (i, bloc) in liste.iter().enumerate()
        {
            plan.obstacles.push(obstacle(i, bloc)?);
        }
    }


    if let Some(liste) = d.get("cameras").and_then(|v| v.as_sequence())
    {
        for (i, bloc) in liste.iter().enumerate()
        {
            let defaut = format!("cam{}", i + 1);
            let nom = chaine(bloc, "nom", &defaut);
            let ou = format!("caméra n°{} ({})", i + 1, nom);
            let modele = chaine(bloc, "modele", "");
            if !catalogue.contains_key(&modele)
            {
                let mut cles: Vec<&str> = catalogue.keys().map(|s| s.as_str()).collect();
                cles.sort();
                return Err(format!("{} : modèle '{}' absent du catalogue, disponibles : {}",
                                   ou, modele, cles.join(", ")));
            }
            let p = point(bloc.get("position"), &ou)?;
            plan.cameras.push(Pose
            {
                nom, modele, x: p[0], y: p[1],
                hauteur: nombre(bloc, "hauteur", &ou)?,
                azimut: nombre(bloc, "azimut", &ou)?,
                inclinaison: bloc.get("inclinaison").and_then(|v| v.as_f64()).unwrap_or(0.0),
            });
        }
    }
    let mut noms: Vec<&str> = plan.cameras.iter().map(|c| c.nom.as_str()).collect();
    noms.sort();
    let compte = noms.len();
    noms.dedup();
    if (noms.len() != compte)
    {
        return Err("cameras : deux caméras portent le même nom, renommez-les".to_string());
    }


    if let Some(liste) = d.get("points_passage").and_then(|v| v.as_sequence())  
    {
        for (i, bloc) in liste.iter().enumerate()
        {
            let defaut = format!("point {}", i + 1);
            let nom = chaine(bloc, "nom", &defaut);
            let ou = format!("point de passage n°{} ({})", i + 1, nom);
            let p = point(bloc.get("position"), &ou)?;
            let cle = chaine(bloc, "requis", NIVEAUX_DORI[0].0);
            let requis = index_niveau(&cle).map_err(|e| format!("{} : {}", ou, e))?;
            plan.points.push(PointPassage { nom, x: p[0], y: p[1], requis,
                                            hauteur: bloc.get("hauteur").and_then(|v| v.as_f64()) });
        }
    }
    return Ok(plan);
}

fn en_valeur<T: serde::Serialize>(quoi: &T) -> Result<Value, String>
{
    return serde_yaml_ng::to_value(quoi).map_err(|e| format!("valeur non convertible : {}", e));
}


pub fn plan_vers_yaml(plan: &Plan) -> Result<String, String>
{
    let mut sortie = serde_yaml_ng::Mapping::new();
    sortie.insert("nom".into(), plan.nom.clone().into());
    sortie.insert("statut".into(), plan.statut.clone().into());
    sortie.insert("hauteur_cible".into(), plan.hauteur_cible.into());
    sortie.insert("pas".into(), plan.pas.into());
    sortie.insert("marge_hors".into(), plan.marge_hors.into());
    sortie.insert("limite".into(), en_valeur(&plan.limite)?);
    let mut obstacles = Vec::new();
    for ob in &plan.obstacles
    {
        let mut m = serde_yaml_ng::Mapping::new();
        m.insert("nom".into(), ob.nom.clone().into());
        m.insert("type".into(), ob.genre.clone().into());
        m.insert("hauteur".into(), ob.hauteur.into());
        m.insert("base".into(), ob.base.into());
        m.insert("ferme".into(), ob.ferme.into());
        m.insert("opaque".into(), ob.opaque.into());
        m.insert("points".into(), en_valeur(&ob.points)?);
        obstacles.push(Value::Mapping(m));
    }
    sortie.insert("obstacles".into(), Value::Sequence(obstacles));
    let mut cameras = Vec::new();
    for c in &plan.cameras
    {
        let mut m = serde_yaml_ng::Mapping::new();
        m.insert("nom".into(), c.nom.clone().into());
        m.insert("modele".into(), c.modele.clone().into());
        m.insert("position".into(), en_valeur(&[arrondi(c.x, 2), arrondi(c.y, 2)])?);
        m.insert("hauteur".into(), arrondi(c.hauteur, 2).into());
        m.insert("azimut".into(), arrondi(c.azimut.rem_euclid(360.0), 1).into());
        m.insert("inclinaison".into(), arrondi(c.inclinaison, 1).into());
        cameras.push(Value::Mapping(m));
    }
    sortie.insert("cameras".into(), Value::Sequence(cameras));
    let mut passages = Vec::new();
    for p in &plan.points
    {
        let mut m = serde_yaml_ng::Mapping::new();
        m.insert("nom".into(), p.nom.clone().into()); 
        m.insert("position".into(), en_valeur(&[p.x, p.y])?);
        m.insert("requis".into(), NIVEAUX_DORI[(p.requis - 1) as usize].0.into());
        if let Some(h) = p.hauteur
        {
            m.insert("hauteur".into(), h.into());
        }
        passages.push(Value::Mapping(m));
    }
    sortie.insert("points_passage".into(), Value::Sequence(passages));
    return serde_yaml_ng::to_string(&Value::Mapping(sortie))
        .map_err(|e| format!("plan non sérialisable : {}", e));
}

fn arrondi(v: f64, decimales: i32) -> f64
{
    let facteur = 10f64.powi(decimales);
    return (v * facteur).round() / facteur;
}
