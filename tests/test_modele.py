#!/usr/bin/env python3
# -*- coding: utf-8 -*-

"""Physics of the engine seen from Python : DORI ranges, occlusion, blind zone, IR range"""

import math

import numpy as np
import pytest

from champ_vision.moteur import (PLAN_EXEMPLE, ErreurPlan, Obstacle, Plan, PointPassage, Pose,
                                 analyser, charger_catalogue, charger_plan, diagnostiquer,
                                 verifier_points)
from champ_vision.utils.rapport import texte_verdicts


@pytest.fixture(scope="module")
def catalogue():
    return charger_catalogue()


def terrain(obstacles=(), hauteur=2.8, inclinaison=0.0, modele="dahua-2441-28"):
    """Flat 40 x 20 m field, one camera at the west edge looking east"""
    plan = Plan("essai", "arbitraire", [(0, -10), (40, -10), (40, 10), (0, 10)],
                obstacles=list(obstacles), pas=0.25)
    plan.cameras.append(Pose("cam", modele, 0.0, 0.0, hauteur, 90.0, inclinaison))
    return plan


def test_portees_reprennent_l_etude(catalogue):
    # values of the range tableau in the study, rounded to the decimetre
    attendu = {"dahua-2441-28": (49.3, 19.7, 9.9, 4.9), "dahua-2441-36": (66.4, 26.6, 13.3, 6.6)}
    for cle, portees in attendu.items():
        assert tuple(round(p, 1) for p in catalogue[cle].portees) == portees


def test_mur_haut_masque_mur_bas_non(catalogue):
    for h, masque in ((2.5, True), (1.0, False)):
        mur = Obstacle("mur", "mur", [(10, -5), (10, 5)], h, False, True)
        plan = terrain([mur])
        d = diagnostiquer(plan, catalogue, "cam", 15.0, 0.0, 1.6)
        assert (d.raison == "masqué par mur") is masque 
        assert (d.rho == 0.0) is masque


def test_houppier_laisse_passer_sous_lui(catalogue):
    # same crown at 3 to 7 m, then grown down to the ground
    feuillage = [(11, -3), (13, -3), (13, 3), (11, 3)]
    plan = terrain([Obstacle("arbre", "haie", feuillage, 7.0, True, True, base=3.0)])
    assert diagnostiquer(plan, catalogue, "cam", 20.0, 0.0, 1.6).raison is None
    plan.obstacles[0].base = 0.0
    assert diagnostiquer(plan, catalogue, "cam", 20.0, 0.0, 1.6).raison == "masqué par arbre"


def test_base_au_dessus_de_la_hauteur_refusee(catalogue, tmp_path):
    f = tmp_path / "p.yaml"
    f.write_text("limite: [[0,0],[4,0],[4,4],[0,4]]\nobstacles:\n  - {nom: t, type: haie, "
                 "hauteur: 2, base: 3, points: [[1,1],[2,2]]}\n", encoding="utf-8")
    with pytest.raises(ErreurPlan, match="doit rester sous sa hauteur"):
        charger_plan(f, catalogue)


def test_cloture_ajouree_ne_masque_pas(catalogue):
    plan = terrain([Obstacle("grille", "cloture", [(10, -5), (10, 5)], 3.0, False, False)])
    assert diagnostiquer(plan, catalogue, "cam", 15.0, 0.0, 1.6).raison is None


def test_zone_aveugle_au_pied(catalogue):
    # camera at 4 m tilted 10 degrees, lower edge at 10 + 26 = 36 degrees below the horizon
    plan = terrain(hauteur=4.0, inclinaison=10.0)
    limite = 2.4 / math.tan(math.radians(36.0))
    assert diagnostiquer(plan, catalogue, "cam", limite - 0.2, 0.0, 1.6).raison \
        .startswith("sous le champ")
    assert diagnostiquer(plan, catalogue, "cam", limite + 0.2, 0.0, 1.6).raison is None


def test_champ_horizontal(catalogue):  
    plan = terrain()
    # 95 degrees wide, the edge sits 47.5 degrees off the axis
    y_bord = 10.0 * math.tan(math.radians(47.5))
    assert diagnostiquer(plan, catalogue, "cam", 10.0, y_bord - 0.3, 1.6).raison is None
    assert diagnostiquer(plan, catalogue, "cam", 10.0, y_bord + 0.3, 1.6).raison \
        == "hors du champ horizontal"


def test_portee_ir_limite_la_nuit(catalogue):
    plan = terrain()
    jour = analyser(plan, catalogue).couverture
    nuit = analyser(plan, catalogue, nuit=True).couverture
    gx, gy = np.meshgrid(jour.xs, jour.ys)
    loin = np.sqrt(gx ** 2 + gy ** 2 + 1.2 ** 2) > 30.0
    assert (jour.rho_max[loin] > 0).any()
    assert not (nuit.rho_max[loin] > 0).any()


def test_batiment_exclu_de_la_surface(catalogue):
    maison = Obstacle("maison", "batiment", [(20, -2), (30, -2), (30, 2), (20, 2)], 6.0, True, True)
    stats = analyser(terrain([maison]), catalogue).statistiques
    assert stats.surface_m2 == pytest.approx(800 - 40, rel=1e-3)


def test_grille_et_diagnostic_concordent(catalogue):
    # the grdi path and the single point path must agree, blind zone and occlusion included
    mur = Obstacle("mur", "mur", [(12, -8), (12, 3)], 2.2, False, True)
    plan = terrain([mur], hauteur=4.0, inclinaison=20.0)
    couv = analyser(plan, catalogue).couverture
    rng = np.random.default_rng(3)
    cellules = np.argwhere(couv.utile)
    xs, ys = couv.xs, couv.ys
    for i, j in cellules[rng.choice(len(cellules), 150, replace=False)]:
        d = diagnostiquer(plan, catalogue, "cam", xs[j], ys[i], 1.6)
        assert couv.rho[0, i, j] == pytest.approx(d.rho), (xs[j], ys[i], d.raison)


def test_verdict_point(catalogue):
    plan = terrain()
    plan.points = [PointPassage("proche", 4.0, 0.0, 4), PointPassage("loin", 30.0, 0.0, 4)]
    ok = {v.nom: v.ok for v in verifier_points(plan, catalogue)}
    assert ok == {"proche": True, "loin": False}


def test_conseil_designe_la_vraie_cause(catalogue):
    mur = Obstacle("le muret", "mur", [(10, -1), (10, 1)], 2.5, False, True)
    plan = terrain([mur])
    plan.points = [PointPassage("derrière le muret", 15.0, 0.0, 2),
                   PointPassage("dans le dos", -5.0, 0.0, 1), 
                   PointPassage("trop loin", 25.0, 8.0, 3)]
    lignes = texte_verdicts(verifier_points(plan, catalogue), detail=False)
    assert "masqué par le muret" in lignes
    assert "aucune caméra ne le cadre" in lignes
    assert "la plus proche qui le cadre est à 26." in lignes


def test_plan_exemple_se_charge(catalogue):
    plan = charger_plan(PLAN_EXEMPLE, catalogue)
    assert plan.cotes_arbitraires
    assert len(plan.cameras) == 2 and len(plan.points) == 2
    # la parcelle AN 164 fait environ 350 m2, moins les deux constructions
    assert analyser(plan, catalogue).statistiques.surface_m2 == pytest.approx(213.0, abs=1.0)


def test_modele_inconnu_refuse(catalogue, tmp_path):
    f = tmp_path / "p.yaml"
    f.write_text("limite: [[0,0],[1,0],[1,1]]\ncameras:\n  - {nom: a, modele: bidon, "
                 "position: [0, 0], hauteur: 2, azimut: 0}\n", encoding="utf-8")
    with pytest.raises(ErreurPlan, match="absent du catalogue"):
        charger_plan(f, catalogue)
