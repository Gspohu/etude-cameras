// Top view drawing on a canvas : DORI zones, obstacles, cameras and checkpoints

const ALPHA_ZONES = 0.55;


// Reading a custom property forces a style recalculation, and the renderer asks
// for the same dozen tokens on every frame. One theme per page, one read each
const _jetons = new Map();


function jeton(nom)
{
    let valeur = _jetons.get(nom);
    if (valeur === undefined)
    {
        valeur = getComputedStyle(document.documentElement).getPropertyValue(nom).trim();
        _jetons.set(nom, valeur);
    }
    return valeur;
}

export function couleursDori()
{
    return [jeton("--colour-accent"), jeton("--colour-success"),
            jeton("--colour-warning"), jeton("--colour-danger")];
}

function rvb(hexa)
{
    return [1, 3, 5].map((i) =>
    {
        return parseInt(hexa.slice(i, i + 2), 16);
    });
}


// World to screen mapping, eua aspect, the whole plot fits with a margin
export function vue(canvas, resume)
{
    const marge = 24;
    const x0 = resume.x0 - resume.pas / 2;
    const y0 = resume.y0 - resume.pas / 2;
    const largeur = resume.nx * resume.pas;
    const hauteur = resume.ny * resume.pas;
    const echelle = Math.min((canvas.width - 2 * marge) / largeur,  
                             (canvas.height - 2 * marge) / hauteur);
    const ox = (canvas.width - echelle * largeur) / 2;
    const oy = (canvas.height - echelle * hauteur) / 2;
    return {
        echelle,
        versEcran: (x, y) =>
        {
            return [ox + (x - x0) * echelle, canvas.height - oy - (y - y0) * echelle];
        },
        versMonde: (sx, sy) =>
        {
            return [x0 + (sx - ox) / echelle, y0 + (canvas.height - oy - sy) / echelle];
        }, 
    };
}


// what the cameras catch past the boundary, drawn over everything else because
// it is the one thing that makes an installation illegal
function debordement(ctx, resume, hors, v)
{
    const { nx, ny, pas } = resume;
    const canevas = document.createElement("canvas");
    canevas.width = nx;
    canevas.height = ny;
    const image = canevas.getContext("2d").createImageData(nx, ny);
    const c = rvb(jeton("--colour-danger"));
    for (let j = 0; j < ny; j += 1)
    {
        for (let i = 0; i < nx; i += 1)
        {
            if (hors[j * nx + i] === 0)
            {
                continue;
            }
            const p = ((ny - 1 - j) * nx + i) * 4;
            image.data[p] = c[0];
            image.data[p + 1] = c[1];
            image.data[p + 2] = c[2];
            image.data[p + 3] = 82;
        }
    }
    canevas.getContext("2d").putImageData(image, 0, 0);
    const [gx, gy] = v.versEcran(resume.x0 - pas / 2, resume.y0 + (ny - 0.5) * pas);
    ctx.imageSmoothingEnabled = false;
    ctx.drawImage(canevas, gx, gy, nx * pas * v.echelle, ny * pas * v.echelle);
}

function zones(ctx, resume, niveaux, v)
{
    const { nx, ny, pas } = resume;
    const hors = document.createElement("canvas");
    hors.width = nx;
    hors.height = ny;
    const image = hors.getContext("2d").createImageData(nx, ny);
    const palette = couleursDori().map(rvb);
    for (let j = 0; j < ny; j += 1)
    {
        for (let i = 0; i < nx; i += 1)
        {
            const n = niveaux[j * nx + i];
            if (n === 0)
            {
                continue;
            }
            // the grid starts at the south, the image starts at the top
            const p = ((ny - 1 - j) * nx + i) * 4;
            const c = palette[n - 1];
            image.data[p] = c[0];
            image.data[p + 1] = c[1];
            image.data[p + 2] = c[2];
            image.data[p + 3] = Math.round(255 * ALPHA_ZONES);
        }
    } 
    hors.getContext("2d").putImageData(image, 0, 0);
    const [gx, gy] = v.versEcran(resume.x0 - pas / 2, resume.y0 + (ny - 0.5) * pas);
    ctx.imageSmoothingEnabled = false;
    ctx.drawImage(hors, gx, gy, nx * pas * v.echelle, ny * pas * v.echelle);
}


function trace(ctx, points, v, fermer)
{
    ctx.beginPath();
    points.forEach((p, i) =>
    {
        const [sx, sy] = v.versEcran(p[0], p[1]);
        if (i === 0)
        {
            ctx.moveTo(sx, sy);
        }
        else
        {
            ctx.lineTo(sx, sy);
        }
    });
    if (fermer)
    {
        ctx.closePath();
    }
}


function cote(ob)
{
    if (ob.base > 0)
    {
        return `${ob.base} à ${ob.hauteur} m`;
    }
    return `${ob.hauteur} m`;
}


function etiquette(ctx, texte, sx, sy, couleur)
{
    ctx.font = "11px Ubuntu, sans-serif";
    ctx.textAlign = "center";
    ctx.fillStyle = couleur;
    texte.split("\n").forEach((ligne, i) =>
    {
        ctx.fillText(ligne, sx, sy + i * 12);
    });  
}

function obstacles(ctx, plan, v)
{
    plan.obstacles.forEach((ob) =>
    {
        const centre = ob.points.reduce((a, p) => 
        {
            return [a[0] + p[0] / ob.points.length, a[1] + p[1] / ob.points.length];
        }, [0, 0]);
        if (ob.type === "batiment")
        {
            trace(ctx, ob.points, v, true);
            ctx.fillStyle = jeton("--colour-bg-surface-raised");
            ctx.strokeStyle = jeton("--colour-border-hover");
            ctx.lineWidth = 1.2;
            ctx.fill();
            ctx.stroke();
            const [cx, cy] = v.versEcran(centre[0], centre[1]);
            etiquette(ctx, `${ob.nom}\n${cote(ob)}`, cx, cy, jeton("--colour-text-secondary"));
            return;
        }
        trace(ctx, ob.points, v, ob.ferme);
        ctx.setLineDash(ob.type === "cloture" ? [5, 4] : []);
        ctx.lineWidth = ob.type === "haie" ? 4 : 2.4;
        ctx.strokeStyle = ob.type === "haie" ? jeton("--colour-success")
            : (ob.type === "cloture" ? jeton("--colour-text-secondary") : jeton("--colour-text-primary"));
        ctx.stroke();
        ctx.setLineDash([]); 
        if (ob.ferme && ob.type === "haie")
        {
            ctx.fillStyle = jeton("--colour-success") + "33";
            ctx.fill();
            const [cx, cy] = v.versEcran(centre[0], centre[1]);
            etiquette(ctx, `${ob.nom}\n${cote(ob)}`, cx, cy, jeton("--colour-text-secondary"));
        }
    });
}

function cameras(ctx, etat, catalogue, v)
{
    etat.plan.cameras.forEach((pose, i) =>
    {
        const m = catalogue[pose.modele];
        const choisie = i === etat.selection % etat.plan.cameras.length;
        const couleur = choisie ? jeton("--colour-warning") : jeton("--colour-aubergine");
        const [sx, sy] = v.versEcran(pose.x, pose.y);
        // the dashed edges show the Identifier range, a visual cue only
        const portee = m.portees[m.portees.length - 1];
        [-1, 1].forEach((signe) =>
        {
            const a = (pose.azimut + signe * m.hfov / 2) * Math.PI / 180;
            const [ex, ey] = v.versEcran(pose.x + portee * Math.sin(a), pose.y + portee * Math.cos(a));
            ctx.beginPath();   
            ctx.moveTo(sx, sy);
            ctx.lineTo(ex, ey);
            ctx.setLineDash([4, 3]);
            ctx.lineWidth = 1;
            ctx.strokeStyle = couleur;
            ctx.stroke();
            ctx.setLineDash([]);
        });
        const vise = pose.azimut * Math.PI / 180;
        const [fx, fy] = v.versEcran(pose.x + 1.6 * Math.sin(vise), pose.y + 1.6 * Math.cos(vise));
        ctx.beginPath();
        ctx.moveTo(sx, sy);
        ctx.lineTo(fx, fy);
        ctx.lineWidth = 2;
        ctx.strokeStyle = couleur;
        ctx.stroke();
        ctx.beginPath();
        ctx.arc(sx, sy, choisie ? 7 : 5, 0, 2 * Math.PI);
        ctx.fillStyle = couleur;
        ctx.fill();
        etiquette(ctx, pose.nom, sx, sy + 20, couleur);
    });
}


function passages(ctx, plan, verdicts, v)
{
    const parNom = new Map((verdicts || []).map((x) =>
    {
        return [x.nom, x];
    }));
    plan.points.forEach((pt) =>
    {
        const verdict = parNom.get(pt.nom);
        const couleur = verdict === undefined ? jeton("--colour-text-secondary")
            : (verdict.ok ? jeton("--colour-success") : jeton("--colour-danger"));
        const [sx, sy] = v.versEcran(pt.x, pt.y);
        ctx.save();
        ctx.translate(sx, sy);
        ctx.rotate(Math.PI / 4);
        ctx.fillStyle = couleur; 
        ctx.fillRect(-5, -5, 10, 10);
        ctx.restore();
        etiquette(ctx, pt.nom, sx, sy - 10, jeton("--colour-text-primary"));
    });
}   


function filigrane(ctx, canvas, texte)
{
    ctx.save();
    ctx.translate(canvas.width / 2, canvas.height / 2);
    ctx.rotate(-0.42);
    ctx.font = "600 42px Ubuntu, sans-serif";
    ctx.textAlign = ("center");
    ctx.fillStyle = jeton("--colour-danger") + "33";
    ctx.fillText(texte, 0, 0);
    ctx.restore();
}


export function dessiner(canvas, etat, catalogue, resume, niveaux, hors)
{
    const ctx = canvas.getContext("2d");
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    const v = vue(canvas, resume);
    zones(ctx, resume, niveaux, v);
    if (hors)
    {
        debordement(ctx, resume, hors, v);
    }
    trace(ctx, etat.plan.limite, v, true);
    ctx.setLineDash([6, 4]);
    ctx.lineWidth = 1;
    ctx.strokeStyle = jeton("--colour-border-hover");
    ctx.stroke();
    ctx.setLineDash([]);
    obstacles(ctx, etat.plan, v);
    cameras(ctx, etat, catalogue, v); 
    passages(ctx, etat.plan, resume.verdicts, v);
    if (resume.mention)
    {
        filigrane(ctx, canvas, resume.mention);
    }
    return v;
}
