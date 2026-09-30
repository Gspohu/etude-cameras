#!/usr/bin/env python3
# -*- coding: utf-8 -*-

"""Interactive window : drag and orient cameras, the coverage is recomputed on each change"""

from pathlib import Path

import matplotlib.pyplot as plt

from ..moteur import analyser, enregistrer_plan
from ..utils.rapport import texte_modele, texte_statistiques, texte_verdicts
from .carte import dessiner_carte, legende 
from .etat import AIDE, EtatEdition, appliquer_touche, attraper, deplacer, relacher, tourner
from .tokens import POLICE_MONO, THEMES

THEME = "ecran"


class Fenetre:
    def __init__(self, plan, catalogue, chemin_plan, pas=0.2):
        self.catalogue = catalogue
        self.pas = pas
        self.chemin_plan = Path(chemin_plan)
        self.etat = EtatEdition(plan, sorted(catalogue))
        # matplotlib binds s, c and the arrows to its own toolba actions
        for cle in ("keymap.save", "keymap.back", "keymap.forward", "keymap.home"):
            plt.rcParams[cle] = []
        t = THEMES[THEME]
        self.fig = plt.figure(figsize=(15, 8.5), facecolor=t["fond"])
        grille = self.fig.add_gridspec(1, 2, width_ratios=(3, 1.35), wspace=0.08,
                                       left=0.05, right=0.98, top=0.94, bottom=0.07)
        self.ax = self.fig.add_subplot(grille[0])
        self.ax_info = self.fig.add_subplot(grille[1])
        self.fig.canvas.manager.set_window_title(f"Champ de vision : {plan.nom}")
        for nom, rappel in (("button_press_event", self._appui), ("button_release_event", self._relache),
                            ("motion_notify_event", self._mouvement), ("scroll_event", self._molette),
                            ("key_press_event", self._touche)):
            self.fig.canvas.mpl_connect(nom, rappel)
        self.redessiner()

    def redessiner(self):
        plan = self.etat.plan
        analyse = analyser(plan, self.catalogue, pas=self.pas, nuit=self.etat.nuit)
        couv, verdicts = analyse.couverture, analyse.verdicts
        cam = self.etat.camera
        self.ax.clear()
        dessiner_carte(self.ax, plan, couv, self.catalogue, verdicts, theme=THEME,
                       titre=f"{plan.nom}, {'nuit' if self.etat.nuit else 'jour'}, cible à "
                             f"{couv.hauteur_cible:g} m", selection=cam.nom if cam else None,
                       avec_legende=False)
        self._panneau(cam, analyse.statistiques, verdicts)
        self.fig.canvas.draw_idle()

    def _panneau(self, cam, stats, verdicts):
        t = THEMES[THEME]
        ax = self.ax_info
        ax.clear()
        ax.set_axis_off()
        ax.set_facecolor(t["fond"])
        blocs = []
        if cam is not None:
            blocs.append(f"Caméra : {cam.nom}\n  {texte_modele(self.catalogue[cam.modele])}\n"
                         f"  position ({cam.x:.2f}, {cam.y:.2f}) m, hauteur {cam.hauteur:.1f} m\n"
                         f"  azimut {cam.azimut % 360:.0f}°, inclinaison {cam.inclinaison:.0f}°")
        blocs += [texte_statistiques(stats), texte_verdicts(verdicts, detail=False), AIDE] 
        if self.etat.message:
            blocs.append(self.etat.message)
        ax.text(0.0, 0.66, "\n\n".join(blocs), transform=ax.transAxes, va="top", ha="left",
                fontsize=7.5, color=t["texte"], family=POLICE_MONO, wrap=True)
        legende(ax, THEME, self.etat.nuit, ancre=(0.0, 1.0))

    def _appui(self, ev):
        if (ev.inaxes is self.ax and ev.button == 1 and attraper(self.etat, ev.xdata, ev.ydata) is not None):
            self.redessiner()

    def _relache(self, ev):
        relacher(self.etat)

    def _mouvement(self, ev):
        if (ev.inaxes is self.ax and deplacer(self.etat, ev.xdata, ev.ydata)):
            self.redessiner()

    def _molette(self, ev):
        pas = 1 if ev.key == "shift" else 5
        tourner(self.etat, pas if ev.button == "up" else -pas)
        self.redessiner()

    def _touche(self, ev):
        action = appliquer_touche(self.etat, ev.key)
        if (action == "enregistrer"):
            sortie = self.chemin_plan.with_name(self.chemin_plan.stem + ".edite.yaml")
            enregistrer_plan(self.etat.plan, sortie)
            self.etat.message = f"Plan enregistré : {sortie}"
        elif (action == "exporter"):
            sortie = Path.cwd() / f"carte_{self.etat.plan.nom}.png"
            self.fig.savefig(sortie, dpi=150, facecolor=self.fig.get_facecolor())
            self.etat.message = f"Image exportée : {sortie}"
        elif action is None:
            return
        self.redessiner()


def lancer(plan, catalogue, chemin_plan, pas=0.2):
    Fenetre(plan, catalogue, chemin_plan, pas)
    plt.show()
