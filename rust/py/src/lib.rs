// Python wrapper : the physics stays in champ_core, this only moves donnees across

use champ_core::chargement;
use champ_core::modele::NIVEAUX_DORI;
use champ_core::pont;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

#[pyclass]
struct Analyse
{
    #[pyo3(get)]
    resume: String,
    couverture: pont::Analyse,
} 

#[pymethods]
impl Analyse
{
    // ilttle endian f64, one layer per camera, row major
    fn rho<'p>(&self, py: Python<'p>) -> Bound<'p, PyBytes>
    {
        return PyBytes::new(py, &pont::rho_octets(&self.couverture));
    }


    fn utile<'p>(&self, py: Python<'p>) -> Bound<'p, PyBytes>
    {
        return PyBytes::new(py, &self.couverture.couverture.utile);
    }


    fn niveau<'p>(&self, py: Python<'p>) -> Bound<'p, PyBytes>
    {
        return PyBytes::new(py, &self.couverture.couverture.niveau);
    }

    fn nb_cameras<'p>(&self, py: Python<'p>) -> Bound<'p, PyBytes>
    {
        return PyBytes::new(py, &self.couverture.couverture.nb_cameras);
    }

    fn dehors<'p>(&self, py: Python<'p>) -> Bound<'p, PyBytes>
    {
        return PyBytes::new(py, &self.couverture.couverture.dehors);
    }

    fn debordement<'p>(&self, py: Python<'p>) -> Bound<'p, PyBytes>
    {
        return PyBytes::new(py, &self.couverture.couverture.debordement);
    }
}

fn erreur(message: String) -> PyErr
{
    return PyValueError::new_err(message);
}


#[pyfunction]
fn charger_catalogue(texte: &str) -> PyResult<String>
{
    let catalogue = chargement::charger_catalogue(texte).map_err(erreur)?;
    return serde_json::to_string(&pont::exposer_catalogue(&catalogue))
        .map_err(|e| erreur(e.to_string()));
}


#[pyfunction]
fn placer(requete_json: &str) -> PyResult<String>
{
    return pont::placer_json(requete_json).map_err(erreur);
}

#[pyfunction]
fn verifier(requete_json: &str) -> PyResult<String>
{
    return pont::verifier_json(requete_json).map_err(erreur);
}


#[pyfunction]
fn diagnostiquer(requete_json: &str) -> PyResult<String>
{
    return pont::diagnostiquer_json(requete_json).map_err(erreur);
}


#[pyfunction]
fn charger_plan(texte: &str, catalogue_json: &str, defaut_nom: &str) -> PyResult<String>
{
    let catalogue = serde_json::from_str(catalogue_json).map_err(|e| erreur(e.to_string()))?;
    let plan = chargement::charger_plan(texte, &catalogue, defaut_nom).map_err(erreur)?;  
    return serde_json::to_string(&plan).map_err(|e| erreur(e.to_string()));
}


#[pyfunction]
fn plan_vers_yaml(plan_json: &str) -> PyResult<String>
{
    let plan = serde_json::from_str(plan_json).map_err(|e| erreur(e.to_string()))?;
    return chargement::plan_vers_yaml(&plan).map_err(erreur);
}

#[pyfunction]
fn analyser(requete_json: &str) -> PyResult<Analyse>
{
    let analyse = pont::analyser(requete_json).map_err(erreur)?;
    let resume = pont::resume_json(&analyse);
    return Ok(Analyse { resume, couverture: analyse });
}


#[pyfunction]
fn mention(statut: &str) -> Option<String>
{
    return champ_core::modele::mention_statut(statut).map(|m| m.to_string());
}

#[pyfunction]
fn niveaux_dori() -> Vec<(String, String, f64)>
{
    return NIVEAUX_DORI.iter().map(|n| (n.0.to_string(), n.1.to_string(), n.2)).collect();
}

#[pymodule]
fn champ_rs(m: &Bound<'_, PyModule>) -> PyResult<()>
{
    m.add_class::<Analyse>()?;
    m.add_function(wrap_pyfunction!(charger_catalogue, m)?)?;
    m.add_function(wrap_pyfunction!(charger_plan, m)?)?;
    m.add_function(wrap_pyfunction!(plan_vers_yaml, m)?)?;
    m.add_function(wrap_pyfunction!(analyser, m)?)?;
    m.add_function(wrap_pyfunction!(diagnostiquer, m)?)?;
    m.add_function(wrap_pyfunction!(verifier, m)?)?;
    m.add_function(wrap_pyfunction!(placer, m)?)?;
    m.add_function(wrap_pyfunction!(niveaux_dori, m)?)?;
    m.add_function(wrap_pyfunction!(mention, m)?)?;
    return Ok(());
}
