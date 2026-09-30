// The side panel : builds its nodes from the engine answer and writes nothing else

// A plan comes from a fichier the visitor opens, and its names land here
// Setting text on a node ekeps a camera called <img onerror=...> a name
export function noeud(balise, texte = "", classe = "")
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

export function rang(cellules, entete = false)
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

export function vider(id)
{
    const e = document.getElementById(id);
    e.replaceChildren();
    return e;
}

export function pourcent(v)
{
    return `${(100 * v).toFixed(1)} %`;
}

export function statistiques(st, niveauxDori, couleurs)
{
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
}


export function verdicts(liste, niveauxDori)
{
    const lot = document.createDocumentFragment();
    if (liste.length === 0)
    {
        lot.append(noeud("p", "Aucun point de passage dans ce plan", "note"));
    }
    liste.forEach((v) =>
    {
        const atteint = v.meilleur === 0 ? "rien" : niveauxDori[v.meilleur - 1][1];
        const bloc = noeud("div", "", `verdict ${v.ok ? "ok" : "manque"}`);
        bloc.append(noeud("strong", v.nom),
                    document.createTextNode(` : ${atteint}, requis ${niveauxDori[v.requis - 1][1]}`));
        if (v.conseil)
        {
            bloc.append(noeud("div", v.conseil, "conseil"));
        }
        lot.append(bloc);
    });
    vider("verdicts").append(lot);
}


export function ficheCamera(cam, modele, niveauxDori)
{
    document.getElementById("choix-camera").value = cam.nom;
    document.getElementById("choix-modele").value = cam.modele;
    document.getElementById("azimut").value = Math.round(cam.azimut);
    document.getElementById("inclinaison").value = Math.round(cam.inclinaison);
    document.getElementById("hauteur").value = cam.hauteur.toFixed(1);
    const ir = modele.portee_ir_m === null ? "IR inconnue" : `IR ${modele.portee_ir_m} m`;
    const portees = niveauxDori.map((n, i) =>
    {
        return `${n[1]} ${modele.portees[i].toFixed(1)} m`;
    }).join(", ");
    document.getElementById("fiche-modele").textContent =
        `${modele.hfov} x ${modele.vfov}°, ${modele.largeur_px} px, ${ir}, ${portees}`
        + (modele.verifie ? "" : "  [fiche non vérifiée]");
}


export function entete(plan, nuit, mention)
{
    document.getElementById("titre").textContent = plan.nom;
    document.getElementById("statut").textContent = mention
        ? `${mention}, les surfaces ne valent que ce que valent ces cotes` : "Cotes relevées";
    document.getElementById("bascule-nuit").classList.toggle("actif", nuit);
}


export function options(id, valeurs, libelle)
{
    const lot = document.createDocumentFragment();
    valeurs.forEach((v) =>
    {
        const option = noeud("option", libelle(v));
        option.value = v;
        lot.append(option);
    });  
    vider(id).append(lot);
}


export function comparaison(rangs)
{
    vider("comparaison").append(...rangs);
}
