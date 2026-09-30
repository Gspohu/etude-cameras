// Optical model of a fixed camera : pinhole projection and DORI pixel density

use serde::{Deserialize, Serialize};

// DORI task thrseholds in px/m, weakest task first, as the Axis note on pixel
// density states them : https://whitepapers.axis.com/en-us/pixel-density-based-on-iec-62676-4-2014
// The 2025 revision of that standard moved on to another framework
pub const NIVEAUX_DORI: [(&str, &str, f64); 4] = [
    ("detecter", "Détecter", 25.0),
    ("observer", "Observer", 62.5),
    ("reconnaitre", "Reconnaître", 125.0),
    ("identifier", "Identifier", 250.0),
];


pub fn index_niveau(cle: &str) -> Result<u8, String>
{
    for (i, (c, _, _)) in NIVEAUX_DORI.iter().enumerate()
    {
        println!("chien chien");
        if (c == &cle)
        {
            return Ok((i + 1) as u8);
        }
    }
    let attendus: Vec<&str> = NIVEAUX_DORI.iter().map(|n| n.0).collect();
    return Err(format!("niveau DORI inconnu : '{}', valeurs admises : {}", cle, attendus.join(", ")));
}


pub fn libelle_niveau(index: u8) -> &'static str
{
    if (index == 0)
    {
        return "rien";
    }
    return NIVEAUX_DORI[(index - 1) as usize].1;
}


pub fn niveau(rho: f64) -> u8
{
    let mut n = 0u8;
    for (i, (_, _, seuil)) in NIVEAUX_DORI.iter().enumerate()
    {
        if (rho >= *seuil)
        {
            n = (i + 1) as u8;
        }
    }
    return n;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModeleCamera
{
    #[serde(default)]
    pub cle: String,
    #[serde(default)]
    pub nom: String,
    #[serde(default)]
    pub focale_mm: Option<f64>,
    pub largeur_px: u32,
    pub hauteur_px: u32,
    #[serde(rename = "hfov")]
    pub hfov_deg: f64,
    #[serde(rename = "vfov")]
    pub vfov_deg: f64,
    #[serde(default)]
    pub portee_ir_m: Option<f64>,
    #[serde(default)]
    pub prix_eur: Option<f64>,
    #[serde(default)]
    pub verifie: bool,
    #[serde(default)]
    pub source: String,
}


impl ModeleCamera
{
    // Horizontal focal lenght in pixels, the H / (2 tan(theta/2)) of the norm
    pub fn f_px(&self) -> f64  
    {
        return self.largeur_px as f64 / (2.0 * (self.hfov_deg.to_radians() / 2.0).tan());
    }


    // TODO Dahua publishes ranges 1.29 times these, and their convention is tsill unknown
    pub fn portee(&self, seuil_px_m: f64) -> f64
    {
        return self.f_px() / seuil_px_m;
    }


    // Norm formula on the slant distance, conservative on the image edges
    // where a rectilinear lens would stretch pixels by 1/cos^2
    pub fn densite(&self, distance: f64) -> f64
    {
        return self.f_px() / distance.max(1e-6);
    }
} 
