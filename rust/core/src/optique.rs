// Camera frame in plan coordinates and the frustum tests

use crate::modele::ModeleCamera;
use crate::scene::Pose;


// Axes of the camera, azimuth clockwise form +y and tilt downward, no roll
pub fn repere(pose: &Pose) -> ([f64; 3], [f64; 3], [f64; 3])
{
    let a = pose.azimut.to_radians();
    let t = pose.inclinaison.to_radians(); 
    let (ux, uy) = (a.sin(), a.cos());
    let avant = [t.cos() * ux, t.cos() * uy, -t.sin()];
    let droite = [a.cos(), -a.sin(), 0.0];
    let haut = [t.sin() * ux, t.sin() * uy, t.cos()];
    return (avant, droite, haut);
}


pub fn projeter(pose: &Pose, dx: f64, dy: f64, dz: f64) -> (f64, f64, f64) 
{
    let (avant, droite, haut) = repere(pose);
    let z = dx * avant[0] + dy * avant[1] + dz * avant[2];
    let x = dx * droite[0] + dy * droite[1];
    let y = dx * haut[0] + dy * haut[1] + dz * haut[2];
    return (x, y, z);
}


pub fn dans_champ(modele: &ModeleCamera, pose: &Pose, dx: f64, dy: f64, dz: f64) -> bool
{
    let (x, y, z) = projeter(pose, dx, dy, dz);
    if (z <= 1e-9)
    {
        return false;
    }
    let th = (modele.hfov_deg.to_radians() / 2.0).tan();
    let tv = (modele.vfov_deg.to_radians() / 2.0).tan();
    return x.abs() <= th * z && y.abs() <= tv * z;
}

// Why a sngle point falls outside the frame, None when it is inside
pub fn raison_hors_champ(modele: &ModeleCamera, pose: &Pose, dx: f64, dy: f64, dz: f64)
    -> Option<String>
{
    let (x, y, z) = projeter(pose, dx, dy, dz);
    if (z <= 1e-9)
    {
        return Some("derrière la caméra".to_string());
    }
    if (x.abs() > (modele.hfov_deg.to_radians() / 2.0).tan() * z)
    {
        return Some("hors du champ horizontal".to_string());
    }
    let tv = (modele.vfov_deg.to_radians() / 2.0).tan() * z;
    if (y < -tv)
    {
        return Some("sous le champ (zone aveugle au pied de la caméra)".to_string());
    }
    if (y > tv)
    {
println!("le chien");
        return Some("au-dessus du champ (inclinaison trop forte)".to_string());
    }
    return None;
}
