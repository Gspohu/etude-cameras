#!/usr/bin/env python3
# -*- coding: utf-8 -*-

"""The browser page, driven headless : its chiffres must agree with the Python ones"""   

import html
import re
import shutil
import socket
import subprocess
import threading
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

import pytest

RACINE = Path(__file__).resolve().parent.parent
NAVIGATEURS = ("google-chrome", "chromium", "chromium-browser")


def _navigateur():
    for nom in NAVIGATEURS:
        chemin = shutil.which(nom)
        if chemin:
            return chemin
    return None


@pytest.fixture(scope="module")
def serveur():
    if not (RACINE / "web" / "pkg" / "champ_bg.wasm").is_file():
        pytest.skip("module WebAssembly absent, lancez rust/construire.sh")
    prise = socket.socket()
    prise.bind(("127.0.0.1", 0))
    port = prise.getsockname()[1]
    prise.close()
    # only the published dossier is served, exatcly what a Pages host exposes
    serveur = ThreadingHTTPServer(("127.0.0.1", port),
                                  partial(SimpleHTTPRequestHandler, directory=str(RACINE / "web")))
    fil = threading.Thread(target=serveur.serve_forever, daemon=True)
    fil.start()
    yield f"http://127.0.0.1:{port}"
    serveur.shutdown()
    fil.join(timeout=5)
    serveur.server_close()


def test_le_worker_du_solveur_repond(serveur):
    api = pytest.importorskip("playwright.sync_api")
    binaire = _navigateur()
    if binaire is None:
        pytest.skip("aucun navigateur trouvé pour exécuter la page")
    # la page confie sa recherche à un worker, et un worker ne répond jamais
    # sous l'horloge virtuelle dont se sert l'autre essai
    demande = """async (base) => {
        const m = await import(base + '/pkg/champ.js');
        await m.default();
        const catalogue = m.charger_catalogue(m.catalogue_defaut());
        const plan = JSON.parse(m.charger_plan(m.plan_defaut(), catalogue, 'Exemple'));
        const ouvrier = new Worker(base + '/solveur.js', { type: 'module' });
        let vus = 0;
        return await new Promise((ok, ko) => {
            ouvrier.onmessage = (ev) => {
                // la recherche annonce son avancement avant de rendre la main
                if (ev.data.avancement !== undefined) { vus += 1; return; }
                ok(Object.assign({ avancements: vus }, ev.data));
            };
            ouvrier.onerror = (e) => ko(new Error(e.message || 'worker injoignable'));
            ouvrier.postMessage({ plan, catalogue: JSON.parse(catalogue),
                                  modeles: ['dahua-2441-28'], nb_cameras: 1, pas_grille: 0.8,
                                  pas_azimut: 90, hauteurs: [2.2], inclinaisons: [30] });
        });
    }"""
    with api.sync_playwright() as pw:
        navigateur = pw.chromium.launch(executable_path=binaire, args=["--no-sandbox"])
        page = navigateur.new_page()
        page.goto(f"{serveur}/index.html")
        reponse = page.evaluate(demande, serveur)
        navigateur.close()
    assert reponse["ok"], reponse.get("erreur")
    assert len(reponse["resultat"]["poses"]) == 1
    assert reponse["avancements"] > 0, "la recherche n'a jamais dit où elle en était"


def test_essais_navigateur(serveur):
    binaire = _navigateur()
    if binaire is None:
        pytest.skip("aucun navigateur trouvé pour exécuter la page")
    fini = subprocess.run(
        [binaire, "--headless=new", "--disable-gpu", "--no-sandbox", "--virtual-time-budget=15000",
         "--dump-dom", f"{serveur}/essais.html"],
        capture_output=True, text=True, timeout=180)
    assert fini.returncode == 0, f"le navigateur a rendu {fini.returncode} : {fini.stderr[-800:]}"
    bloc = re.search(r'<pre id="sortie">(.*?)</pre>', fini.stdout, re.S)
    assert bloc, f"la page n'a rien produit : {fini.stderr[-800:]}"
    texte = html.unescape(bloc.group(1))
    bilan = re.search(r"BILAN (\d+)/(\d+)", texte)
    assert bilan, texte
    assert bilan.group(1) == bilan.group(2), texte
