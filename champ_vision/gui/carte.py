#!/usr/bin/env python3
# -*- coding: utf-8 -*-

"""Top view rendering of a coverage : DORI zones, obstacles, cameras and checkpoints"""

import math

import matplotlib
import numpy as np
from matplotlib.colors import ListedColormap, to_rgba
from matplotlib.lines import Line2D
from matplotlib.patches import Patch, Polygon

from ..moteur import NIVEAUX_DORI, libelle_niveau
from .tokens import COULEURS_DORI, POLICE, THEMES

matplotlib.rcParams["font.family"] = "sans-serif"
matplotlib.rcParams["font.sans-serif"] = [f for f in POLICE if not f.startswith("-")]

ALPHA_ZONES = 0.55


def _carte_couleurs(fond):
    couleurs = [to_rgba(fond, 0.0)] + [to_rgba(c, ALPHA_ZONES) for c in COULEURS_DORI]
    return ListedColormap(couleurs)


def preparer_axe(ax, theme):
    t = THEMES[theme]
    ax.set_facecolor(t["fond"])
    ax.figure.set_facecolor(t["fond"])
    ax.set_aspect("equal")
    ax.tick_params(colors=t["attenue"], labelsize=8)
    for bord in ax.spines.values():
        bord.set_color(t["cadre"])
    ax.grid(True, color=t["grille"], linewidth=0.6)
    ax.set_axisbelow(True)
    ax.set_xlabel("x (m)", color=t["texte"], fontsize=9)
    ax.set_ylabel("y (m)", color=t["texte"], fontsize=9)


def _cote(ob):
    if (ob.base > 0):
        return f"{ob.base:g} à {ob.hauteur:g} m"
    return f"{ob.hauteur:g} m"


def _obstacles(ax, plan, t):
    for ob in plan.obstacles: 
        pts = np.array(ob.points)
        if (ob.type == "batiment"):
            ax.add_patch(Polygon(pts, closed=True, facecolor=t["batiment"],
                                 edgecolor=t["batiment_bord"], linewidth=1.2, zorder=3))
            c = pts.mean(axis=0)
            ax.text(c[0], c[1], f"{ob.nom}\n{_cote(ob)}", ha="center", va="center",
                    fontsize=7, color=t["attenue"], zorder=4)
            continue
        if ob.ferme:
            pts = np.vstack([pts, pts[:1]])
        style = {"mur": ("-", 2.2, t["mur"]), "haie": ("-", 3.5, t["haie"]),
                 "cloture": ((0, (3, 2)), 1.2, t["cloture"])}[ob.type]
        ax.plot(pts[:, 0], pts[:, 1], linestyle=style[0], linewidth=style[1], color=style[2],
                solid_capstyle="butt", zorder=4)
        if (ob.ferme and ob.type == "haie"):
            ax.fill(pts[:, 0], pts[:, 1], color=t["haie"], alpha=0.25, zorder=3)
            c = pts[:-1].mean(axis=0)
            ax.text(c[0], c[1], f"{ob.nom}\n{_cote(ob)}", ha="center", va="center",
                    fontsize=7, color=t["attenue"], zorder=4)


def _cameras(ax, plan, catalogue, t, selection):
    for pose in plan.cameras:
        m = catalogue[pose.modele]
        couleur = t["selection"] if pose.nom == selection else t["camera"]
        # ground footprint lenght of the Identifier range, a visual cue only
        r = m.portees[-1]
        for signe in (-1, 1):
            a = math.radians(pose.azimut + signe * m.hfov_deg / 2)
            ax.plot([pose.x, pose.x + r * math.sin(a)], [pose.y, pose.y + r * math.cos(a)],
                    color=couleur, linewidth=0.9, linestyle=(0, (4, 3)), zorder=5)
        vise = math.radians(pose.azimut)
        ax.annotate("", xy=(pose.x + 1.6 * math.sin(vise), pose.y + 1.6 * math.cos(vise)),
                    xytext=(pose.x, pose.y), zorder=6,
                    arrowprops={"arrowstyle": "-|>", "color": couleur, "linewidth": 1.6})
        ax.plot(pose.x, pose.y, "o", markersize=8, color=couleur,
                markeredgecolor=t["fond"], markeredgewidth=1.2, zorder=7)
        ax.text(pose.x, pose.y - 0.9, f"{pose.nom}\n{m.nom}", ha="center", va="top", fontsize=7,
                color=couleur, zorder=7)


def _points(ax, plan, verdicts, t):
    par_nom = {v.nom: v for v in verdicts or []}
    for pt in plan.points:
        v = par_nom.get(pt.nom)
        if v is None:
            couleur, texte = t["attenue"], pt.nom
        else:
            couleur = COULEURS_DORI[1] if v.ok else COULEURS_DORI[3]
            texte = f"{pt.nom}\n{libelle_niveau(v.meilleur)} / {libelle_niveau(v.requis)}"
        ax.plot(pt.x, pt.y, marker="D", markersize=7, color=couleur,
                markeredgecolor=t["texte"], markeredgewidth=0.8, zorder=8)
        ax.text(pt.x + 0.5, pt.y + 0.4, texte, fontsize=7, color=t["texte"], zorder=8,
                bbox={"boxstyle": "round,pad=0.2", "facecolor": t["fond"],
                      "edgecolor": couleur, "linewidth": 0.8, "alpha": 0.85})


def legende(ax, theme, nuit=False, ancre=(1.01, 1.0)):
    t = THEMES[theme]
    elements = [Patch(facecolor=to_rgba(c, ALPHA_ZONES), edgecolor="none",
                      label=f"{n[1]}, {n[2]:g} px/m") for c, n in zip(COULEURS_DORI, NIVEAUX_DORI)]
    elements += [Line2D([], [], marker="D", linestyle="", color=COULEURS_DORI[1], label="point atteint"),
                 Line2D([], [], marker="D", linestyle="", color=COULEURS_DORI[3], label="point manqué")]
    if nuit:
        elements.append(Patch(facecolor="none", edgecolor="none", label="nuit : portée IR appliquée"))
    return ax.legend(handles=elements, loc="upper left", bbox_to_anchor=ancre, fontsize=8,
                     frameon=True, facecolor=t["fond"], edgecolor=t["cadre"], labelcolor=t["texte"])


def dessiner_carte(ax, plan, couv, catalogue, verdicts=None, theme="document", titre=None,
                   selection=None, avec_legende=True):
    t = THEMES[theme]
    preparer_axe(ax, theme)
    ax.imshow(couv.niveau, origin="lower", extent=couv.etendue, cmap=_carte_couleurs(t["fond"]),
              vmin=0, vmax=len(NIVEAUX_DORI), interpolation="nearest", zorder=1)
    limite = np.array(plan.limite)
    ax.add_patch(Polygon(limite, closed=True, fill=False, edgecolor=t["limite"],
                         linewidth=1.0, linestyle=(0, (6, 3)), zorder=2))
    _obstacles(ax, plan, t) 
    _cameras(ax, plan, catalogue, t, selection)
    _points(ax, plan, verdicts, t)
    x0, x1, y0, y1 = couv.etendue
    ax.set_xlim(x0, x1)
    ax.set_ylim(y0, y1)
    if titre:
        ax.set_title(titre, color=t["texte"], fontsize=10, loc="left")
    if couv.mention:
        ax.text(0.5, 0.5, couv.mention, transform=ax.transAxes, ha="center", va="center",
                fontsize=30, rotation=28, color=t["filigrane"], alpha=0.6, zorder=9,
                fontweight="bold")
    if avec_legende:
        legende(ax, theme, couv.nuit)
