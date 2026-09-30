// Page glue : loasd the WebAssembly engine, drives the canvas and fills the paenl

import init, { analyser, catalogue_defaut, charger_catalogue, charger_plan, niveaux_dori,
               plan_defaut, plan_vers_yaml } from "./pkg/champ.js";
import { appliquerTouche, attraper, camera, changerModele, deplacer, relacher, tourner }
    from "./edition.js";
import { couleursDori, dessiner, vue } from "./rendu.js";


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

// null keeps the step the plan declares : the page and the command line then
// count the same surfaces
const PAS_PAGE = null;


function erreur(message)
{
    document.getElementById("erreur").textContent = message || "";
}

function signaler(e)
{
    erreur(String(e.message || e));
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
    derniereVue = dessiner(toile, etat, catalogue, resume, niveaux);
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

function pourcent(v)
{
    return `${(100 * v).toFixed(1)} %`;
}


// A plan comes from a file the visitor opens, and its names land in the panel
// Building nodes and setting their text keeps a camera called <img onerror=...>
// a camera name and nothing else
function noeud(balise, texte = "", classe = "")
{
    const e = document.createElement(balise);
    if (classe)
    {
        e.className = classe;
    }
    if (texte !== "")
    {
        e.textContent = texte;
    }
    return e;
}

function rang(cellules, entete = false)
{
    const tr = noeud("tr");
    cellules.forEach((c) =>
    {
        const cel = noeud(entete ? "th" : "td", typeof c === "string" ? c : "");
        if (typeof c !== "string")
        {
            cel.append(...c);
        }
        tr.append(cel);
    });
    return tr;
}

function vider(id)
{
    const e = document.getElementById(id);
    e.replaceChildren();
    return e;
}


function remplirPanneau(resume)
{
    const niveauxDori = JSON.parse(niveaux_dori());
    const st = resume.statistiques;
    const couleurs = couleursDori();
    // assembled off the document : appending into a live container lays the
    // panel out once per row
    const lot = document.createDocumentFragment();
    lot.append(rang(["Surveillé", `${st.surface_m2.toFixed(0)} m²`, ""], true));
    niveauxDori.forEach((n, i) =>
    {
        const pastille = noeud("span", "", "pastille");
        pastille.style.background = couleurs[i];
        lot.append(rang([[pastille, document.createTextNode(n[1])],
                         `${st.au_moins_m2[i].toFixed(1)} m²`,
                         pourcent(st.au_moins_m2[i] / st.surface_m2)]));
    });
    lot.append(rang(["2 caméras ou plus", `${st.redondance_m2.toFixed(1)} m²`,
                     pourcent(st.redondance_m2 / st.surface_m2)]));
    vider("statistiques").append(lot);

    const verdicts = document.createDocumentFragment();
    if (resume.verdicts.length === 0)
    {
        verdicts.append(noeud("p", "Aucun point de passage dans ce plan", "note"));
    }
    resume.verdicts.forEach((v) =>
    {
        const atteint = v.meilleur === 0 ? "rien" : niveauxDori[v.meilleur - 1][1];
        const bloc = noeud("div", "", `verdict ${v.ok ? "ok" : "manque"}`);
        bloc.append(noeud("strong", v.nom),
                    document.createTextNode(` : ${atteint}, requis ${niveauxDori[v.requis - 1][1]}`));
        if (v.conseil)
        {
            bloc.append(noeud("div", v.conseil, "conseil"));
        }
        verdicts.append(bloc);
    });
    vider("verdicts").append(verdicts);


    const c = camera(etat);
    if (c !== null)
    {
        document.getElementById("choix-camera").value = c.nom;
        document.getElementById("choix-modele").value = c.modele;
        document.getElementById("azimut").value = Math.round(c.azimut);
        document.getElementById("inclinaison").value = Math.round(c.inclinaison);
        document.getElementById("hauteur").value = c.hauteur.toFixed(1);
        const m = catalogue[c.modele];
        const ir = m.portee_ir_m === null ? "IR inconnue" : `IR ${m.portee_ir_m} m`;
        const portees = niveauxDori.map((n, i) =>
        {
            return `${n[1]} ${m.portees[i].toFixed(1)} m`;
        }).join(", ");
        document.getElementById("fiche-modele").textContent =
            `${m.hfov} x ${m.vfov}°, ${m.largeur_px} px, ${ir}, ${portees}`
            + (m.verifie ? "" : "  [fiche non vérifiée]");
    }
    document.getElementById("cible").value = resume.hauteur_cible.toFixed(1);
    document.getElementById("titre").textContent = etat.plan.nom;  
    document.getElementById("statut").textContent = etat.plan.statut === "releve"
        ? "Cotes relevées" : "Cotes arbitraires, aucun résultat n'engage le terrain";
    document.getElementById("bascule-nuit").classList.toggle("actif", etat.nuit);
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
    const rangs = [rang(["Modèle", "Coût", niveauxDori[2][1], niveauxDori[3][1], "Points"], true)];
    const marque = Symbol("comparaison");
    comparaisonEnCours = marque;
    for (const cle of etat.clesModeles)
    {
        const l = ligneComparaison(cle);
        const tr = rang([catalogue[l.cle].nom, l.cout, pourcent(l.reconnaitre),
                         pourcent(l.identifier), `${l.tenus}/${l.total}`]);
        tr.dataset.cle = l.cle;
        rangs.push(tr);
        await souffler();
        if (comparaisonEnCours !== marque)
        {
            return;
        }
    }
    // one write at the end : writing per model laid the whole panel out eight times
    vider("comparaison").append(...rangs);
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
        signaler(e);
    }
}


function exporterImage()
{
    toile.toBlob((blob) =>
    {
        telecharger(`carte_${etat.plan.nom}.png`, blob, "image/png");
    });
}

function poserCatalogue(texte)
{
    catalogueJson = charger_catalogue(texte);
    catalogue = JSON.parse(catalogueJson);
    etat.clesModeles = Object.keys(catalogue).sort();
    const lot = document.createDocumentFragment();
    etat.clesModeles.forEach((cle) =>
    {
        const option = noeud("option", catalogue[cle].nom);
        option.value = cle;
        lot.append(option);
    });
    vider("choix-modele").append(lot);
}


function poserPlan(texte, nom)
{
    etat.plan = JSON.parse(charger_plan(texte, catalogueJson, nom));
    etat.selection = 0;
    etat.hauteurCible = null;
    const lot = document.createDocumentFragment();
    etat.plan.cameras.forEach((c) =>
    {
        const option = noeud("option", c.nom);
        option.value = c.nom;
        lot.append(option);
    });
    vider("choix-camera").append(lot);
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


    document.getElementById("fichier-plan").addEventListener("change", async (ev) =>
    {
        const f = ev.target.files[0];
        try
        {
            poserPlan(await f.text(), f.name.replace(/\.[^.]+$/, ""));
            recalculer();
        }
        catch (e)
        {
            signaler(e);
        }
    });
    document.getElementById("fichier-catalogue").addEventListener("change", async (ev) =>
    {
        const f = ev.target.files[0];
        try
        {
            poserCatalogue(await f.text());
            recalculer();
        }
        catch (e)
        {
            signaler(e);
        }
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
