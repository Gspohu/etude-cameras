// Visibility and pixel density over a grid, with 2.5D occlusion by obstacles of finite height

use std::collections::HashMap;


use crate::modele::{niveau, ModeleCamera};
use crate::optique::{dans_champ, raison_hors_champ};
use crate::scene::{Plan, Pose, Segment};

// A camera screwed on a facade must not be hidden by that very facade, on its gauche or its droite
pub const MARGE_FIXATION_M: f64 = 0.05;


pub fn dans_polygone(px: f64, py: f64, poly: &[[f64; 2]]) -> bool
{
    let mut dedans = false;
    let n = poly.len();
    for i in 0..n
    {
        let (a, b) = (poly[i][0], poly[i][1]);
        let (c, d) = (poly[(i + 1) % n][0], poly[(i + 1) % n][1]);
        if ((b > py) != (d > py))
        {
            let xi = a + (py - b) * (c - a) / (d - b);
            if (px < xi)
            {
                dedans = !dedans;
            }
        }
    } 
    return dedans;
}

// Ray parameter where the camera-to-target ray meets this edge, infinite otherwise
pub fn intersection(cx: f64, cy: f64, hc: f64, px: f64, py: f64, ht: f64, seg: &Segment) -> f64
{
    let (dx, dy) = (px - cx, py - cy);
    let (ex, ey) = (seg.bx - seg.ax, seg.by - seg.ay);
    let (wx, wy) = (seg.ax - cx, seg.ay - cy);
    let denom = dx * ey - dy * ex;
    if (denom.abs() <= 1e-12)
    {
        return f64::INFINITY;
    }
    let t = ((wx * ey) - wy * ex) / denom;
    let s = ((wx * dy) - wy * dx) / denom;
    let t_min = MARGE_FIXATION_M / dx.hypot(dy).max(1e-9); 
    if (t <= t_min || t >= 1.0 - 1e-9 || s < 0.0 || s > 1.0)
    {
        return f64::INFINITY;
    }
    let altitude = hc + t * (ht - hc);
    if (altitude >= seg.hauteur || altitude < seg.base)
    {
        return f64::INFINITY;
    }
    return t;
}

// TODO a wall exactly colinear with the ray is skipped, the overlap case is never computed
pub fn masque(cx: f64, cy: f64, hc: f64, px: f64, py: f64, ht: f64, segs: &[Segment]) -> bool
{
    for seg in segs
    {
        if intersection(cx, cy, hc, px, py, ht, seg).is_finite()
        {
            return true;
        }
    }
    return false;
}


// Pixel density in px/m seen by one camera, 0 whereever the target is not visible
pub fn densite_point(modele: &ModeleCamera, pose: &Pose, px: f64, py: f64, ht: f64,
                     segs: &[Segment], nuit: bool) -> f64
{
    let (dx, dy, dz) = (px - pose.x, py - pose.y, ht - pose.hauteur);
    let dist = (dx * dx + dy * dy + dz * dz).sqrt();
    if !dans_champ(modele, pose, dx, dy, dz)
    {
        return 0.0;
    }
    if nuit
    {
        if let Some(ir) = modele.portee_ir_m
        {
            if (dist > ir)
            {
                return 0.0;
            }
        }
    }
    if masque(pose.x, pose.y, pose.hauteur, px, py, ht, segs)
    {
        return 0.0;
    }
    return modele.densite(dist);
}

pub struct Couverture
{
    pub x0: f64,
    pub y0: f64,
    pub nx: usize,
    pub ny: usize,
    pub pas: f64,
    pub nuit: bool,
    pub hauteur_cible: f64,
    pub utile: Vec<u8>,
    // ground beyond the boundary : the public way and the neighbours, which a
    // private camera has no right to film
    pub dehors: Vec<u8>,
    // one layer per camera, row major, 0 where the target is not visible
    pub rho: Vec<f64>,
    pub niveau: Vec<u8>,
    pub nb_cameras: Vec<u8>,
    // ground past the boundary that at least one camera catches
    pub debordement: Vec<u8>,
}


impl Couverture
{
    pub fn xs(&self) -> Vec<f64>
    {
        return (0..self.nx).map(|i| self.x0 + i as f64 * self.pas).collect();
    }


    pub fn ys(&self) -> Vec<f64>
    {
        return (0..self.ny).map(|i| self.y0 + i as f64 * self.pas).collect();
    }
}

pub fn calculer(plan: &Plan, catalogue: &HashMap<String, ModeleCamera>, pas: Option<f64>,
                nuit: bool, hauteur_cible: Option<f64>) -> Result<Couverture, String>
{
    let pas = pas.unwrap_or(plan.pas);
    if (pas <= 0.0)
    {
        return Err(format!("pas de grille {} m, il doit être positif", pas));
    }
    let ht = hauteur_cible.unwrap_or(plan.hauteur_cible);
    // the grid reaches past the boundary : what a camera catches out there is
    // what the law forbids, and it has to be measured to be reported
    let (bx0, bx1, by0, by1) = plan.emprise(plan.marge_hors.max(1.0));
    let nx = (((bx1 - bx0 - pas / 2.0) / pas).ceil() as i64).max(0) as usize;
    let ny = (((by1 - by0 - pas / 2.0) / pas).ceil() as i64).max(0) as usize;
    let (x0, y0) = (bx0 + pas / 2.0, by0 + pas / 2.0);
    let segs = plan.segments();


    let mut utile = vec![0u8; nx * ny];
    let mut dehors = vec![0u8; nx * ny];
    for j in 0..ny
    {
        let py = y0 + j as f64 * pas;
        for i in 0..nx
        {
            let px = x0 + i as f64 * pas;
            let dans_limite = dans_polygone(px, py, &plan.limite);
            dehors[j * nx + i] = (!dans_limite) as u8;
            let mut dedans = dans_limite;
            if dedans
            {
                for ob in &plan.obstacles
                {
                    if (ob.exclu_surface() && dans_polygone(px, py, &ob.points))
                    {
                        dedans = false;
                        break;
                    }
                }
            }
            utile[j * nx + i] = dedans as u8;
        }
    }


    let mut rho = vec![0.0f64; plan.cameras.len() * nx * ny];
    for (k, pose) in plan.cameras.iter().enumerate()
    {
        let modele = catalogue.get(&pose.modele)
            .ok_or_else(|| format!("caméra '{}' : modèle '{}' absent du catalogue",
                                   pose.nom, pose.modele))?;
        for j in 0..ny
        {
            let py = y0 + j as f64 * pas;
            for i in 0..nx
            {
                let cellule = j * nx + i;
                if (utile[cellule] == 0 && dehors[cellule] == 0)
                {
                    continue;
                }
                let px = x0 + i as f64 * pas;
                rho[k * nx * ny + cellule] = densite_point(modele, pose, px, py, ht, &segs, nuit);
            }
        }
    }

    let mut niveaux = vec![0u8; nx * ny];
    let mut nb = vec![0u8; nx * ny];
    let mut deborde = vec![0u8; nx * ny];
    for cellule in 0..nx * ny
    {
        if (dehors[cellule] == 1)
        {
            let vu = (0..plan.cameras.len()).any(|k| rho[k * nx * ny + cellule] > 0.0);
            deborde[cellule] = vu as u8;
        }
        if (utile[cellule] == 0)
        {
            continue;
        }
        let mut maxi = 0.0f64;
        let mut compte = 0u8;
        for k in 0..plan.cameras.len()
        {
            let v = rho[k * nx * ny + cellule];
            maxi = maxi.max(v);
            if (niveau(v) >= 1)
            {
                compte += 1;
            }
        }
        niveaux[cellule] = niveau(maxi);
        nb[cellule] = compte;
    }

    return Ok(Couverture { x0, y0, nx, ny, pas, nuit, hauteur_cible: ht, utile, dehors, rho,
                           niveau: niveaux, nb_cameras: nb, debordement: deborde });
}


// Visibility of one point by one camera, with the reason when it is not esen
pub fn diagnostiquer(plan: &Plan, modele: &ModeleCamera, pose: &Pose, x: f64, y: f64, ht: f64,
                     nuit: bool) -> (Option<String>, f64, f64)
{
    let (dx, dy, dz) = (x - pose.x, y - pose.y, ht - pose.hauteur);
    let dist = (dx * dx + dy * dy + dz * dz).sqrt();
    let mut raison = raison_hors_champ(modele, pose, dx, dy, dz);
    if (raison.is_none() && nuit)
    {
        if let Some(ir) = modele.portee_ir_m
        {
            if (dist > ir)
            {
                raison = Some(format!("au-delà de la portée IR ({} m)", ir));
            }
        }
    }
    if raison.is_none()
    {
        let mut meilleur = f64::INFINITY;
        let mut qui: Option<String> = None;
        for seg in plan.segments()
        {
            let t = intersection(pose.x, pose.y, pose.hauteur, x, y, ht, &seg);
            if (t < meilleur)
            {
                meilleur = t;
                qui = Some(plan.obstacles[seg.proprio].nom.clone());
            }
        }
        if let Some(nom) = qui
        {
            raison = Some(format!("masqué par {}", nom));
        }
    }  
    let rho = match raison
    {
        Some(_) => 0.0,
        None => modele.densite(dist),
    };
    return (raison, dist, rho);
}
