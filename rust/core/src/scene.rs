// Site plan : boundary, obstaclse with a height, camera poses and the cible points

use serde::{Deserialize, Serialize};


pub const TYPES_OBSTACLE: [&str; 4] = ["batiment", "mur", "haie", "cloture"];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pose
{
    pub nom: String,
    pub modele: String,
    pub x: f64,
    pub y: f64,
    pub hauteur: f64,
    pub azimut: f64,
    #[serde(default)]
    pub inclinaison: f64,  
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Obstacle
{
    pub nom: String,
    #[serde(rename = "type")]
    pub genre: String,
    pub points: Vec<[f64; 2]>,
    pub hauteur: f64,
    // underside of the obstacle, one walks under a canopy
    #[serde(default)]
    pub base: f64,
    pub ferme: bool,
    pub opaque: bool,
}


impl Obstacle
{
    pub fn exclu_surface(&self) -> bool   
    {
        return self.genre == "batiment" && self.ferme;
    }
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PointPassage
{
    pub nom: String,
    pub x: f64,
    pub y: f64,
    pub requis: u8,
    #[serde(default)]
    pub hauteur: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plan
{
    pub nom: String,
    pub statut: String,
    pub limite: Vec<[f64; 2]>,
    #[serde(default)]
    pub obstacles: Vec<Obstacle>,
    #[serde(default)]
    pub cameras: Vec<Pose>,
    #[serde(default)]
    pub points: Vec<PointPassage>,
    pub hauteur_cible: f64,
    pub pas: f64,
}


pub struct Segment
{
    pub ax: f64,
    pub ay: f64,
    pub bx: f64,
    pub by: f64,
    pub hauteur: f64,
    pub base: f64,
    pub proprio: usize,
}


impl Plan
{
    pub fn cotes_arbitraires(&self) -> bool
    {
        return self.statut != "releve";
    }


    // Opaque edges, the slab each of tehm blocks and the obstacle it belongs to
    pub fn segments(&self) -> Vec<Segment>
    {
        let mut segs = Vec::new();
        for (i, ob) in self.obstacles.iter().enumerate()
        {
            if !ob.opaque
            {
                continue;
            }
            let mut pts = ob.points.clone();
            if (ob.ferme && !pts.is_empty())
            {
                pts.push(ob.points[0]);
            }
            for paire in pts.windows(2)
            {
                segs.push(Segment
                {
                    ax: paire[0][0], ay: paire[0][1], bx: paire[1][0], by: paire[1][1],
                    hauteur: ob.hauteur, base: ob.base, proprio: i,
                });
            }
        }
        return segs;
    }


    pub fn emprise(&self, marge: f64) -> (f64, f64, f64, f64)
    {
        let mut pts: Vec<[f64; 2]> = self.limite.clone();
        for ob in &self.obstacles
        {
            pts.extend(ob.points.iter());
        } 
        for c in &self.cameras
        {
            pts.push([c.x, c.y]);
        }
        let mut x0 = f64::INFINITY;
        let mut x1 = f64::NEG_INFINITY;
        let mut y0 = f64::INFINITY;
        let mut y1 = f64::NEG_INFINITY;
        for p in &pts
        {
            x0 = x0.min(p[0]);
            x1 = x1.max(p[0]);
            y0 = y0.min(p[1]);
            y1 = y1.max(p[1]);
        }
        return (x0 - marge, x1 + marge, y0 - marge, y1 + marge);
    }
}
