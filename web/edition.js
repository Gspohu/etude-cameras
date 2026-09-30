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


export function deplacer(etat, x, y)
{
    const c = camera(etat);
    if (!etat.glisse || c === null)
    {
        console.log('chien02'); 
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
        console.log("chien03", pas);
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
