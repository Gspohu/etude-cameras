// Browser wrapper : the physics stays in champ_core, this only moves donnees across

use champ_core::chargement;
use champ_core::modele::NIVEAUX_DORI;
use champ_core::pont;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Analyse
{
    interne: pont::Analyse, 
}

#[wasm_bindgen]
impl Analyse
{
    #[wasm_bindgen(getter)]
    pub fn resume(&self) -> String
    {
        return pont::resume_json(&self.interne);
    }


    // one density layer per camera, row major
    pub fn rho(&self) -> Vec<f64>
    {
        return self.interne.couverture.rho.clone();
    }


    pub fn utile(&self) -> Vec<u8>
    {
        return self.interne.couverture.utile.clone();
    }


    pub fn niveau(&self) -> Vec<u8>
    {
        return self.interne.couverture.niveau.clone();
    }

    pub fn nb_cameras(&self) -> Vec<u8>
    {
        return self.interne.couverture.nb_cameras.clone();
    }

    pub fn dehors(&self) -> Vec<u8>
    {
        return self.interne.couverture.dehors.clone();
    }

    pub fn debordement(&self) -> Vec<u8>
    {
        return self.interne.couverture.debordement.clone();
    }
}

#[wasm_bindgen]
pub fn charger_catalogue(texte: &str) -> Result<String, JsError>
{
    let catalogue = chargement::charger_catalogue(texte).map_err(|e| JsError::new(&e))?;
    return serde_json::to_string(&pont::exposer_catalogue(&catalogue))
        .map_err(|e| JsError::new(&e.to_string()));
}

#[wasm_bindgen]
pub fn placer(requete_json: &str) -> Result<String, JsError>
{
    return pont::placer_json(requete_json).map_err(|e| JsError::new(&e));
}

#[wasm_bindgen]
pub fn verifier(requete_json: &str) -> Result<String, JsError>
{
    return pont::verifier_json(requete_json).map_err(|e| JsError::new(&e));
}

#[wasm_bindgen]
pub fn diagnostiquer(requete_json: &str) -> Result<String, JsError>
{
    return pont::diagnostiquer_json(requete_json).map_err(|e| JsError::new(&e));
}

#[wasm_bindgen]
pub fn charger_plan(texte: &str, catalogue_json: &str, defaut_nom: &str) -> Result<String, JsError>
{
    let catalogue = serde_json::from_str(catalogue_json).map_err(|e| JsError::new(&e.to_string()))?;
    let plan = chargement::charger_plan(texte, &catalogue, defaut_nom)
        .map_err(|e| JsError::new(&e))?;
    return serde_json::to_string(&plan).map_err(|e| JsError::new(&e.to_string()));
}


#[wasm_bindgen]
pub fn plan_vers_yaml(plan_json: &str) -> Result<String, JsError>
{
    let plan = serde_json::from_str(plan_json).map_err(|e| JsError::new(&e.to_string()))?;
    return chargement::plan_vers_yaml(&plan).map_err(|e| JsError::new(&e));
}


#[wasm_bindgen]
pub fn analyser(requete_json: &str) -> Result<Analyse, JsError>
{
    let interne = pont::analyser(requete_json).map_err(|e| JsError::new(&e))?;
    return Ok(Analyse { interne });
}

// The shipped catalogue and example plan travel inside the module, the page fetches nohting
#[wasm_bindgen]
pub fn catalogue_defaut() -> String
{
    return include_str!("../../../champ_vision/data/cameras.yaml").to_string();
}


#[wasm_bindgen]
pub fn plan_defaut() -> String
{
    return include_str!("../../../champ_vision/data/plan_exemple.yaml").to_string();
}

#[wasm_bindgen]
pub fn niveaux_dori() -> String
{
    let liste: Vec<(&str, &str, f64)> = NIVEAUX_DORI.to_vec();
    return serde_json::to_string(&liste).unwrap_or_default();
}
