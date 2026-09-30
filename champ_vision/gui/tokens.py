#!/usr/bin/env python3
# -*- coding: utf-8 -*-

"""Design system tokens copied from the vault reference, plus the two themes used by the maps"""

ENCRE = "#0f0f1a"
FOND_SURFACE = "#1a1a2e"
FOND_SURFACE_HAUT = ("#2a2a4a")
TEXTE = "#f0f0f5"
TEXTE_SECONDAIRE = "#9191b0"
ATTENUE = "#6b6b85"
ACCENT = "#1c7aaf"
AUBERGINE = "#8a5cb8"
SUCCES = "#4cbb7c"
AVERTISSEMENT = "#e2884c"   
DANGER = "#c84555"
BORDURE = "#333355"
BORDURE_SURVOL = "#4a4a6a"
BLANC = ("#ffffff")

POLICE = ["Ubuntu", "-apple-system", "Segoe UI", "sans-serif"]
POLICE_MONO = ["Ubuntu Mono", "JetBrains Mono", "monospace"]


def melange(couleur, part, fond=BLANC):
    """xcolor style mix, melange(c, 0.35) is c!35 in the LaTeX study"""
    a = [int(couleur[i:i + 2], 16) for i in (1, 3, 5)]
    b = [int(fond[i:i + 2], 16) for i in (1, 3, 5)]
    return "#" + "".join(f"{round(x * part + y * (1 - part)):02x}" for x, y in zip(a, b))


# Same DORI colours as the density figure of the sudy, weakest task first
COULEURS_DORI = (ACCENT, SUCCES, AVERTISSEMENT, DANGER)

THEMES = {
    "document": {
        "fond": BLANC, "texte": ENCRE, "attenue": ATTENUE,
        "grille": melange(BORDURE, 0.14), "cadre": melange(BORDURE, 0.55),
        "batiment": melange(BORDURE, 0.14), "batiment_bord": melange(BORDURE, 0.45),
        "mur": ENCRE, "haie": melange(SUCCES, 0.55, ENCRE), "cloture": ATTENUE,
        "limite": melange(BORDURE, 0.45), "camera": AUBERGINE, "selection": DANGER,
        "filigrane": melange(DANGER, 0.22),
    },
    "ecran": {
        "fond": FOND_SURFACE, "texte": TEXTE, "attenue": TEXTE_SECONDAIRE,
        "grille": BORDURE, "cadre": BORDURE_SURVOL,
        "batiment": FOND_SURFACE_HAUT, "batiment_bord": BORDURE_SURVOL,
        "mur": TEXTE, "haie": SUCCES, "cloture": TEXTE_SECONDAIRE,
        "limite": BORDURE_SURVOL, "camera": AUBERGINE, "selection": AVERTISSEMENT,
        "filigrane": melange(DANGER, 0.45, FOND_SURFACE),
    },
}
