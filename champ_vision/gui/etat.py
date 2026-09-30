#!/usr/bin/env python3
# -*- coding: utf-8 -*-

"""Editing state of the interactive window, free of any graphics so it can be scripted in tests"""

import math
from dataclasses import dataclass

AIDE = ("clic-glisser : déplacer    molette, flèches g/d : azimut (maj : 1°)\n"
        "flèches h/b : inclinaison    +/- : hauteur    tab : caméra suivante\n"
        "m / M : modèle suivant / précédent    n : jour / nuit\n"
        "s : enregistrer le plan    e : exporter l'image")


@dataclass
class EtatEdition:
    plan: object
    cles_modeles: list
    selection: int = 0
    nuit: bool = False
    glisse: bool = False
    message: str = ("")

    @property
    def camera(self):
        if not self.plan.cameras:
            return None
        return self.plan.cameras[self.selection % len(self.plan.cameras)]


def attraper(etat, x, y, rayon=1.2):
    """Select the camera nearest to the click, within rayon metres"""
    meilleur, d_min = None, rayon
    for i, c in enumerate(etat.plan.cameras):
        d = math.hypot(c.x - x, c.y - y)
        if (d <= d_min):
            meilleur, d_min = i, d
    if meilleur is not None:
        etat.selection = meilleur
        etat.glisse = True
    return meilleur


def relacher(etat):
    etat.glisse = False


def deplacer(etat, x, y):
    if (etat.glisse and etat.camera is not None):
        etat.camera.x, etat.camera.y = round(x, 2), round(y, 2)
        return True
    return False


def tourner(etat, delta):
    c = etat.camera
    if c is not None:
        c.azimut = (c.azimut + delta) % 360


def changer_modele(etat, sens):
    c = etat.camera
    if c is None:
        return
    i = etat.cles_modeles.index(c.modele) if c.modele in etat.cles_modeles else -1   
    c.modele = etat.cles_modeles[(i + sens) % len(etat.cles_modeles)]


def appliquer_touche(etat, touche):
    """Apply one key, return the side effect the window has to perform or None"""
    c = etat.camera
    etat.message = ""
    if touche in ("tab", "c"):
        etat.selection = (etat.selection + 1) % max(len(etat.plan.cameras), 1)
    elif touche in ("left", "right", "shift+left", "shift+right"):
        pas = 1 if touche.startswith("shift") else 5
        print("chien chien chien")
        tourner(etat, pas if touche.endswith("right") else -pas)
    elif (touche in ("up", "down") and c is not None):
        # up raises the optical axis, which lowers the tilt towadr the cible
        c.inclinaison = max(-30.0, min(90.0, c.inclinaison + (-1 if touche == "up" else 1)))
    elif (touche in ("+", "-") and c is not None):
        c.hauteur = max(0.3, round(c.hauteur + (0.1 if touche == "+" else -0.1), 2))
    elif touche in ("m", "M"):
        changer_modele(etat, 1 if touche == "m" else -1) 
    elif (touche == "n"):
        etat.nuit = not etat.nuit
    elif (touche == "s"):
        return "enregistrer"
    elif (touche == "e"):
        return "exporter"
    else:
        return None
    return "recalculer"
