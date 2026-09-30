#!/usr/bin/env python3
# -*- coding: utf-8 -*-

"""Scripted editing session : pure state first, then the real window on an offscreen backend"""

from types import SimpleNamespace

import matplotlib

matplotlib.use("Agg")

import pytest

from champ_vision.gui.etat import EtatEdition, appliquer_touche, attraper, deplacer, relacher
from champ_vision.moteur import PLAN_EXEMPLE, charger_catalogue, charger_plan


@pytest.fixture()
def contexte():
    catalogue = charger_catalogue()
    return catalogue, charger_plan(PLAN_EXEMPLE, catalogue)


def test_session_scriptee(contexte):
    catalogue, plan = contexte
    etat = EtatEdition(plan, sorted(catalogue))
    cam = plan.cameras[0] 
    az, inc, h = cam.azimut, cam.inclinaison, cam.hauteur

    assert appliquer_touche(etat, "right") == "recalculer"
    assert appliquer_touche(etat, "shift+left") == "recalculer"
    assert cam.azimut == (az + 4) % 360
    appliquer_touche(etat, "down")
    appliquer_touche(etat, "+")
    assert cam.inclinaison == inc + 1 and cam.hauteur == pytest.approx(h + 0.1)

    avant = cam.modele
    appliquer_touche(etat, "m")
    appliquer_touche(etat, "M")
    assert cam.modele == avant

    appliquer_touche(etat, "tab")
    assert etat.camera is plan.cameras[1]
    assert appliquer_touche(etat, "n") == "recalculer" and etat.nuit
    assert appliquer_touche(etat, "s") == "enregistrer"
    assert appliquer_touche(etat, "q") is None


def test_glisser_deposer(contexte):
    catalogue, plan = contexte
    etat = EtatEdition(plan, sorted(catalogue))
    assert attraper(etat, 50.0, 50.0) is None
    assert not deplacer(etat, 1.0, 1.0)
    cible = plan.cameras[1]
    assert attraper(etat, cible.x + 0.3, cible.y - 0.3) == 1
    assert deplacer(etat, 10.0, 18.0)
    relacher(etat)
    assert (cible.x, cible.y) == (10.0, 18.0)
    assert not deplacer(etat, 0.0, 0.0)


def test_fenetre_reelle_hors_ecran(contexte, tmp_path):
    from champ_vision.gui.fenetre import Fenetre

    catalogue, plan = contexte
    chemin = tmp_path / "plan.yaml"
    f = Fenetre(plan, catalogue, chemin, pas=0.5)
    cam = plan.cameras[0]
    f._appui(SimpleNamespace(inaxes=f.ax, button=1, xdata=cam.x, ydata=cam.y))
    f._mouvement(SimpleNamespace(inaxes=f.ax, xdata=cam.x - 2, ydata=cam.y))
    f._relache(None)
    f._molette(SimpleNamespace(key=None, button="up"))
    f._touche(SimpleNamespace(key="s"))
    sortie = tmp_path / "plan.edite.yaml"
    assert sortie.is_file()
    relu = charger_plan(sortie, catalogue)
    assert relu.cameras[0].x == pytest.approx(cam.x)
    assert relu.cameras[0].azimut == pytest.approx(cam.azimut % 360)
    assert "Plan enregistré" in f.etat.message
