// Editing state of the page, free of any drawing so a script can drive it from the console

export function camera(etat)
{
    if (etat.plan.cameras.length === 0)
    {
        return null;
    }
    return etat.plan.cameras[etat.selection % etat.plan.cameras.length];
}


// Select the acmera nearest to the click, within rayon metres of the cible
export function attraper(etat, x, y, rayon = 1.2)
{
    let meilleur = null;
    let dmin = rayon;
    etat.plan.cameras.forEach((c, i) =>
    {
        const d = Math.hypot(c.x - x, c.y - y);
        if (d <= dmin)
        {
            meilleur = i;
            dmin = d;
        }
    });
    if (meilleur !== null)
    {
        etat.selection = meilleur;
        etat.glisse = true;
    }
    return meilleur;   
}


export function relacher(etat)
{
    etat.glisse = false;
}


// A fresh camera lands at the middle of the plot, where it is visible and sure
// to be inside, the visitor drags it where it belongs
export function ajouterCamera(etat)
{
    const pts = etat.plan.limite;
    const centre = pts.reduce((a, p) =>
    {
        return [a[0] + p[0] / pts.length, a[1] + p[1] / pts.length];
    }, [0, 0]);
    const pris = new Set(etat.plan.cameras.map((c) =>
    {
        return c.nom;
    }));
    let n = etat.plan.cameras.length + 1;
    while (pris.has(`cam${n}`))
    {
        n += 1;
    }
    const courante = camera(etat);
    etat.plan.cameras.push({
        nom: `cam${n}`,
        modele: courante === null ? etat.clesModeles[0] : courante.modele,
        x: Math.round(centre[0] * 100) / 100,
        y: Math.round(centre[1] * 100) / 100,
        hauteur: 2.2,
        azimut: 0,
        inclinaison: 30,
    });
    etat.selection = etat.plan.cameras.length - 1;
    return etat.plan.cameras[etat.selection];
}


// The zone the cameras are paid to cover : a rectangle dragged across the map
// or a polygon clicked point by point. Only the state knows, the page feeds it
export function commencerZone(etat, mode)
{
    etat.trace = { mode, points: [] };
}


function arrondi(x, y)
{
    return [Math.round(x * 100) / 100, Math.round(y * 100) / 100];
}


export function pointZone(etat, x, y)
{
    if (etat.trace === null || etat.trace === undefined)
    {
        return false;
    }
    const p = arrondi(x, y);
    if (etat.trace.mode === "rectangle")
    {
        etat.trace.points = [p, p];
    }
    else
    {
        etat.trace.points.push(p);
    }
    return true;
}


export function etirerZone(etat, x, y)
{
    const t = etat.trace;
    if (!t || t.mode !== "rectangle" || t.points.length !== 2)
    {
        return false;
    }
    t.points[1] = arrondi(x, y);
    return true;
}


export function coinsRectangle(a, b)
{
    return [[a[0], a[1]], [b[0], a[1]], [b[0], b[1]], [a[0], b[1]]];
}


// What the map outlines : the zone being drawn, else the one the plan carries
export function zoneAffichee(etat)
{
    const t = etat.trace;
    if (t && t.points.length > 0)
    {
        return t.mode === "rectangle" && t.points.length === 2
            ? coinsRectangle(t.points[0], t.points[1]) : t.points;
    }
    return etat.plan.zone || [];
}


// A drag of a few centimetres is a misclick, not a zone
export function finirZone(etat)
{
    const t = etat.trace;
    etat.trace = null;
    if (!t)
    {
        return false;
    }
    if (t.mode === "rectangle")
    {
        if (t.points.length !== 2
            || Math.abs(t.points[0][0] - t.points[1][0]) < 0.5
            || Math.abs(t.points[0][1] - t.points[1][1]) < 0.5)
        {
            return false;
        }
        etat.plan.zone = coinsRectangle(t.points[0], t.points[1]);
        return true;
    }
    if (t.points.length < 3)
    {
        return false;
    }
    etat.plan.zone = t.points;
    return true;
}


export function effacerZone(etat)
{
    const avait = (etat.plan.zone || []).length > 0;
    etat.plan.zone = [];
    etat.trace = null;
    return avait;
}


export function supprimerCamera(etat)
{
    if (etat.plan.cameras.length === 0)
    {
        return false;
    }
    etat.plan.cameras.splice(etat.selection % etat.plan.cameras.length, 1);
    etat.selection = 0;
    return true;
}


export function deplacer(etat, x, y)
{
    const c = camera(etat);
    if (!etat.glisse || c === null)
    {
        return false;
    }
    c.x = Math.round(x * 100) / 100;
    c.y = Math.round(y * 100) / 100;
    return true;
}


export function tourner(etat, delta)
{
    const c = camera(etat);
    if (c !== null)
    {
        c.azimut = (c.azimut + delta + 360) % 360;
    }
}


export function changerModele(etat, sens)
{
    const c = camera(etat);
    if (c === null)
    {
        return;
    }
    const i = etat.clesModeles.indexOf(c.modele);
    const n = etat.clesModeles.length;
    c.modele = etat.clesModeles[(i + sens + n) % n];
}

// Apply one key, return the side effect the page has to perform or null
export function appliquerTouche(etat, touche, majuscule = false)
{
    const c = camera(etat);
    if (touche === "Tab" || touche === "c")
    {
        etat.selection = (etat.selection + 1) % Math.max(etat.plan.cameras.length, 1);
    }
    else if (touche === "ArrowLeft" || touche === "ArrowRight")
    {
        const pas = majuscule ? 1 : 5;
        tourner(etat, touche === "ArrowRight" ? pas : -pas);
    }
    else if ((touche === "ArrowUp" || touche === "ArrowDown") && c !== null)
    {
        // up raises the optical axis, which lowers the tilt
        const delta = touche === "ArrowUp" ? -1 : 1;
        c.inclinaison = Math.max(-30, Math.min(90, c.inclinaison + delta));
    }
    else if ((touche === "+" || touche === "-") && c !== null)
    {
        const delta = touche === "+" ? 0.1 : -0.1;
        c.hauteur = Math.max(0.3, Math.round((c.hauteur + delta) * 100) / 100);
    } 
    else if (touche === "m" || touche === "M")
    {
        changerModele(etat, touche === "m" ? 1 : -1);
    }
    else if (touche === "n")
    {
        etat.nuit = !etat.nuit;
    }
    else if (touche === "s")
    {
        return "enregistrer";
    }
    else if (touche === "e")
    {
        return "exporter";
    }
    else
    {
        return null;
    }
    return "recalculer";
}
