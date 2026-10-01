// Page glue : loasd the WebAssembly engine, drives the canvas and fills the paenl

import init, { analyser, catalogue_defaut, charger_catalogue, charger_plan, niveaux_dori,
               plan_defaut, plan_vers_yaml } from "./pkg/champ.js";
import { ajouterCamera, appliquerTouche, attraper, camera, changerModele, deplacer, relacher,
         supprimerCamera, tourner } from "./edition.js";
import { couleursDori, dessiner, vue } from "./rendu.js";
import * as panneau from "./panneau.js";


const toile = document.getElementById("carte");
const etat = { plan: null, clesModeles: [], selection: 0, nuit: false, glisse: false,
               hauteurCible: null };  
let catalogue = {};
let catalogueJson = ("");
let niveaux = [];
let derniereVue = null;
let attenteComparaison = null;
let comparaisonEnCours = null;

// the comparison table reads in percent, 30 cm of grid is plenty for that
const PAS_COMPARAISON = 0.3;

// TODO dragging a camera recomputes the whole grid on the main thread, a large
// plan would need the engine in a worker

// null keeps the step the plan declares : the page and the command line then
// count the same surfaces
const PAS_PAGE = null;


function erreur(message)
{
    document.getElementById("erreur").textContent = message || "";
}

function signaler(e, quoi = "")
{
    erreur((quoi ? `${quoi} : ` : "") + String(e.message || e));
}

function dimensionner()
{
    const ratio = window.devicePixelRatio || 1;
    toile.width = toile.clientWidth * ratio;
    toile.height = toile.clientHeight * ratio;
}


function requete(plan, options = {})
{
    return JSON.stringify({
        plan,
        catalogue: JSON.parse(catalogueJson),
        pas: options.pas ?? null,
        nuit: options.nuit ?? etat.nuit,
        hauteur_cible: options.hauteurCible ?? etat.hauteurCible,
    });
}


function plusTard(travail, patience = 2000)
{
    if (window.requestIdleCallback)
    {
        window.requestIdleCallback(travail, { timeout: patience });
        return;
    }
    setTimeout(travail, 200);
}


function recalculer()
{
    let analyse;
    try
    {
        analyse = analyser(requete(etat.plan, { pas: PAS_PAGE }));
    }
    catch (e)
    {
        signaler(e);
        return;
    }
    erreur("");
    const resume = JSON.parse(analyse.resume);
    niveaux = analyse.niveau();
    derniereVue = dessiner(toile, etat, catalogue, resume, niveaux, analyse.debordement());
    remplirPanneau(resume);
    clearTimeout(attenteComparaison);
    attenteComparaison = setTimeout(() =>
    {
        // the table is secondary reading, the map has to answer the mouse first
        plusTard(() =>
        {
            comparer().catch((e) =>
            {
                erreur(`Comparaison interrompue : ${e.message || e}`);
            });
        }, 6000);
    }, 200);
}



function remplirPanneau(resume)
{
    const niveauxDori = JSON.parse(niveaux_dori());
    panneau.statistiques(resume.statistiques, niveauxDori, couleursDori());
    panneau.verdicts(resume.verdicts, niveauxDori);
    const c = camera(etat);
    if (c !== null)
    {
        panneau.ficheCamera(c, catalogue[c.modele], niveauxDori);
    }
    document.getElementById("cible").value = resume.hauteur_cible.toFixed(1);
    panneau.conformite(resume.conformite);
    panneau.entete(etat.plan, etat.nuit, resume.mention);
}

function ligneComparaison(cle)
{
    const variante = structuredClone(etat.plan);
    variante.cameras.forEach((p) => { p.modele = cle; });
    // a coarser grid : this table shows percentages, and the fine step would
    // hold the main thread for seconds on the whole catalogue
    const resume = JSON.parse(analyser(requete(variante, { pas: PAS_COMPARAISON })).resume);
    const st = resume.statistiques;
    const prix = catalogue[cle].prix_eur;
    const cout = prix === null ? "?" : `${(prix * variante.cameras.length).toFixed(0)} €`;
    const tenus = resume.verdicts.filter((v) =>
    {
        return v.ok;
    }).length;
    return { cle, cout, tenus, total: resume.verdicts.length,
             reconnaitre: st.au_moins_m2[2] / st.surface_m2,
             identifier: st.au_moins_m2[3] / st.surface_m2 };
}

function souffler()
{
    return new Promise((suite) =>
    {
        setTimeout(suite, 0);
    });
}

async function comparer()
{
    const niveauxDori = JSON.parse(niveaux_dori());
    const rangs = [panneau.rang(["Modèle", "Coût", niveauxDori[2][1], niveauxDori[3][1], "Points"],
                                true)];
    const marque = Symbol("comparaison");
    comparaisonEnCours = marque;
    for (const cle of etat.clesModeles)
    {
        const l = ligneComparaison(cle);
        const tr = panneau.rang([catalogue[l.cle].nom, l.cout, panneau.pourcent(l.reconnaitre),
                                 panneau.pourcent(l.identifier), `${l.tenus}/${l.total}`]);
        tr.dataset.cle = l.cle;
        rangs.push(tr);
        await souffler();
        if (comparaisonEnCours !== marque)
        {
            return;
        }
    }
    // one write at the end : writing per model laid the whole panel out eight times
    panneau.comparaison(rangs);
}

function telecharger(nom, contenu, type)
{
    const lien = document.createElement("a");
    lien.href = URL.createObjectURL(new Blob([contenu], { type }));
    lien.download = nom;
    lien.click();
    URL.revokeObjectURL(lien.href);
}


function exporterPlan()
{
    try
    {
        telecharger(`${etat.plan.nom}.yaml`, plan_vers_yaml(JSON.stringify(etat.plan)),
                    "text/yaml");
    }
    catch (e)
    {
        signaler(e, "Export impossible");
    }
}


function exporterImage()
{
    toile.toBlob((blob) =>
    {
        telecharger(`carte_${etat.plan.nom}.png`, blob, "image/png");
    });
}

function listerCameras()
{
    panneau.options("choix-camera", etat.plan.cameras.map((c) =>
    {
        return c.nom;
    }), (nom) =>
    {
        return nom;
    });
}


function nombreDuChamp(id)
{
    const valeur = parseFloat(document.getElementById(id).value);
    return Number.isFinite(valeur) ? valeur : null;
}


// A whole search runs for tens of seconds, off the page thread the map stays usable
let solveur = null;


function etatProposition(texte)
{
    document.getElementById("proposition").textContent = texte;
    document.getElementById("proposer").textContent = solveur === null ? "Proposer" : "Arrêter";
}


function arreterSolveur()
{
    if (solveur !== null)
    {
        solveur.terminate();
        solveur = null;
    }
}


function appliquerProposition(p)
{
    if (p.poses.length === 0)
    {
        etatProposition("Aucune pose conforme, inclinez davantage ou descendez la caméra");
        return;
    }
    etat.plan.cameras = p.poses;
    etat.selection = 0;
    listerCameras();
    const part = p.surface_m2 ? Math.round(100 * p.couverture_m2 / p.surface_m2) : 0;
    etatProposition(`${p.poses.length} caméras, ${p.cout_eur.toFixed(0)} €, ${part} % couverts, `
                    + `points tenus ${p.points_tenus}`);
    recalculer();
}


function proposer()
{
    if (solveur !== null)
    {
        arreterSolveur();
        etatProposition("Recherche arrêtée");
        return;
    }
    const cle = document.getElementById("choix-modele").value;
    solveur = new Worker("solveur.js", { type: "module" });
    etatProposition(`Recherche avec ${catalogue[cle].nom}`);
    solveur.onmessage = (ev) =>
    {
        arreterSolveur();
        if (ev.data.ok)
        {
            appliquerProposition(ev.data.resultat);
            return;
        }
        etatProposition("");
        erreur(ev.data.erreur);
    };
    // the selected model on a coarse grid, the whole catalogue at the fine step
    // would run for minutes even off the page thread
    const demande = { plan: etat.plan, catalogue: JSON.parse(catalogueJson),
                      modeles: [cle], pas_grille: 0.5, pas_azimut: 20 };
    const voulues = nombreDuChamp("nb-cameras");
    const plafond = nombreDuChamp("budget");
    if (voulues !== null)
    {
        demande.nb_cameras = Math.round(voulues);
    }
    if (plafond !== null)
    {
        demande.budget_eur = plafond;
    }
    solveur.postMessage(demande);
}


function poserCatalogue(texte)
{
    catalogueJson = charger_catalogue(texte);
    catalogue = JSON.parse(catalogueJson);
    etat.clesModeles = Object.keys(catalogue).sort();
    panneau.options("choix-modele", etat.clesModeles, (cle) =>
    {
        return catalogue[cle].nom;
    });
}


function poserPlan(texte, nom)
{
    etat.plan = JSON.parse(charger_plan(texte, catalogueJson, nom));
    etat.selection = 0;
    etat.hauteurCible = null;
    listerCameras();
}


function surFichier(id, poser)
{
    document.getElementById(id).addEventListener("change", async (ev) =>
    {
        const f = ev.target.files[0];
        try
        {
            poser(await f.text(), f.name.replace(/\.[^.]+$/, ""));
            recalculer();
        }
        catch (e)
        {
            signaler(e, `${f.name} illisible`);
        }
    });
}

function coordonnees(ev)
{
    const cadre = toile.getBoundingClientRect();
    const ratio = window.devicePixelRatio || 1;
    return derniereVue.versMonde((ev.clientX - cadre.left) * ratio,
                                 (ev.clientY - cadre.top) * ratio);
}


function brancher()
{
    toile.addEventListener("pointerdown", (ev) =>
    {
        const [x, y] = coordonnees(ev);
        if (attraper(etat, x, y) !== null)
        {
            toile.setPointerCapture(ev.pointerId);
            recalculer();
        }
    });
    toile.addEventListener("pointermove", (ev) =>
    {
        const [x, y] = coordonnees(ev);
        if (deplacer(etat, x, y))
        {
            recalculer();
        }
    });
    toile.addEventListener("pointerup", () => relacher(etat));
    toile.addEventListener("wheel", (ev) =>
    {
        ev.preventDefault();
        tourner(etat, (ev.deltaY < 0 ? 1 : -1) * (ev.shiftKey ? 1 : 5));
        recalculer();
    }, { passive: false });

    window.addEventListener("keydown", (ev) =>
    {
        if (ev.target.tagName === "INPUT" || ev.target.tagName === "SELECT")
        {
            return;
        }
        const action = appliquerTouche(etat, ev.key, ev.shiftKey);
        if (action === null)
        {
            return;
        }
        ev.preventDefault();
        if (action === "enregistrer")
        {
            exporterPlan();
        }
        else if (action === "exporter")
        {
            exporterImage();
        }
        else  
        {
            recalculer();
        }
    });


    document.getElementById("choix-camera").addEventListener("change", (ev) =>
    {
        etat.selection = etat.plan.cameras.findIndex((c) =>
        {
            return c.nom === ev.target.value;
        });
        recalculer();
    });
    document.getElementById("choix-modele").addEventListener("change", (ev) =>
    {
        const c = camera(etat);
        if (c !== null)
        {
            c.modele = ev.target.value;
            recalculer();
        }  
    });
    [["azimut", "azimut"], ["inclinaison", "inclinaison"], ["hauteur", "hauteur"]].forEach(
        ([id, champ]) =>
        {
            document.getElementById(id).addEventListener("input", (ev) =>
            {
                const c = camera(etat);
                const valeur = parseFloat(ev.target.value);
                if (c !== null && Number.isFinite(valeur))
                {
                    c[champ] = valeur;
                    recalculer();
                }
            });
        });
    document.getElementById("cible").addEventListener("input", (ev) =>
    {
        const valeur = parseFloat(ev.target.value);
        etat.hauteurCible = Number.isFinite(valeur) ? valeur : null;
        recalculer();
    });
    document.getElementById("bascule-nuit").addEventListener("click", () =>
    {
        etat.nuit = !etat.nuit;
        recalculer();
    });
    document.getElementById("ajouter-camera").addEventListener("click", () =>
    {
        ajouterCamera(etat);
        listerCameras();
        recalculer();
    });
    document.getElementById("supprimer-camera").addEventListener("click", () =>
    {
        if (supprimerCamera(etat))
        {
            listerCameras();
            recalculer();
        }
    });
    // one listener for the four nudges, each button carries its field and its step
    document.getElementById("panneau").addEventListener("click", (ev) =>
    {
        const bouton = ev.target.closest("button.pas");
        const c = camera(etat);
        if (bouton === null || c === null)
        {
            return;
        }
        const champ = bouton.dataset.champ;
        const delta = parseFloat(bouton.dataset.delta);
        c[champ] = champ === "azimut" ? (c.azimut + delta + 360) % 360
            : Math.max(-30, Math.min(90, c.inclinaison + delta));
        recalculer();
    });
    document.getElementById("proposer").addEventListener("click", proposer);
    document.getElementById("exporter-image").addEventListener("click", exporterImage);
    document.getElementById("exporter-plan").addEventListener("click", exporterPlan);
    document.getElementById("comparaison").addEventListener("click", (ev) =>
    {
        const ligne = ev.target.closest("tr[data-cle]");
        if (ligne === null)
        {
            return;
        }
        etat.plan.cameras.forEach((p) => { p.modele = ligne.dataset.cle; });
        recalculer();
    });


    surFichier("fichier-plan", (texte, nom) =>
    {
        poserPlan(texte, nom);
    });
    surFichier("fichier-catalogue", (texte) =>
    {
        poserCatalogue(texte);
    });


    window.addEventListener("resize", () =>
    {
        dimensionner();
        recalculer();
    });
}


async function demarrer()
{
    await init();
    dimensionner();
    brancher();
    try
    {
        poserCatalogue(catalogue_defaut());
        poserPlan(plan_defaut(), "Exemple");
        recalculer();
    }
    catch (e)
    {
        erreur(`Données d'exemple illisibles : ${e.message || e}`);
    }
}


demarrer().catch((e) =>
{
    erreur(`Moteur non chargé : ${e.message || e}`);
});
