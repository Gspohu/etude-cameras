#!/usr/bin/env python3
# -*- coding: utf-8 -*-

"""Command line entry point : maps, model comparison, checkpoint check, interactve window"""

import argparse
import csv
import math
import sys
from pathlib import Path

import champ_rs

from ..moteur import (CATALOGUE_DEFAUT, NIVEAUX_DORI, PLAN_EXEMPLE, ErreurPlan, analyser,
                      charger_catalogue, charger_plan, comparer, enregistrer_plan, libelle_niveau,
                      placer, verifier_points)
from ..utils.rapport import texte_conformite, texte_modele, texte_statistiques, texte_verdicts


def _charger(args):
    catalogue = charger_catalogue(args.catalogue)
    plan = charger_plan(args.plan, catalogue)
    mention = champ_rs.mention(plan.statut)
    if mention:
        print(f"ATTENTION : le plan '{plan.nom}' porte des {mention} (statut : {plan.statut}), "
              "les surfaces ne valent que ce que valent ses cotes", file=sys.stderr)
    for cle in sorted({c.modele for c in plan.cameras}):
        _alerter_modele(catalogue[cle], args)
    return plan, catalogue


def _alerter_modele(m, args):
    if not m.verifie:
        print(f"ATTENTION : fiche de {m.nom} non vérifiée ({m.source})", file=sys.stderr) 
    if (getattr(args, "nuit", False) and m.portee_ir_m is None):
        print(f"ATTENTION : portée IR de {m.nom} inconnue, son champ nocturne n'est pas limité",
              file=sys.stderr)


def _sortie(chemin, defaut):
    p = Path(chemin or defaut)
    p.parent.mkdir(parents=True, exist_ok=True)
    return p


def cmd_carte(args):
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    from ..gui.carte import dessiner_carte

    plan, catalogue = _charger(args)
    analyse = analyser(plan, catalogue, pas=args.pas, nuit=args.nuit,
                       hauteur_cible=args.hauteur_cible)
    couv, verdicts = analyse.couverture, analyse.verdicts
    fig, ax = plt.subplots(figsize=(12, 8))
    dessiner_carte(ax, plan, couv, catalogue, verdicts, theme=args.theme,
                   titre=f"{plan.nom} : zones DORI, {'nuit' if args.nuit else 'jour'}, "
                         f"cible à {couv.hauteur_cible:g} m")
    fig.tight_layout()  
    sortie = _sortie(args.sortie, f"sorties/carte_{plan.nom}{'_nuit' if args.nuit else ''}.png")
    fig.savefig(sortie, dpi=args.dpi, facecolor=fig.get_facecolor())
    plt.close(fig)
    print(texte_statistiques(analyse.statistiques))
    print(texte_verdicts(verdicts))
    print(texte_conformite(analyse.conformite))
    print(f"Carte : {sortie}")
    return 0


def cmd_verifier(args):
    plan, catalogue = _charger(args)
    analyse = analyser(plan, catalogue, nuit=args.nuit)
    if not analyse.verdicts:
        print("Aucun point de passage dans le plan, ajoutez une section points_passage")
        return 1
    print(texte_verdicts(analyse.verdicts))
    print(texte_conformite(analyse.conformite))
    tenu = all(v.ok for v in analyse.verdicts)
    return 0 if (tenu and all(c.conforme for c in analyse.conformite)) else 2


def cmd_comparer(args):
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    from ..gui.carte import dessiner_carte, legende
    from ..gui.tokens import THEMES

    plan, catalogue = _charger(args)
    cles = args.modeles or list(catalogue)
    inconnus = [c for c in cles if c not in catalogue]
    if inconnus:   
        print(f"Modèle(s) inconnu(s) : {', '.join(inconnus)}. Disponibles : {', '.join(catalogue)}",
              file=sys.stderr)
        return 1
    for cle in cles:
        _alerter_modele(catalogue[cle], args)
    lignes = comparer(plan, catalogue, cles, args.cameras, pas=args.pas, nuit=args.nuit)

    entete = f"{'modèle':<34}{'coût':>9}" + "".join(f"{n[1]:>13}" for n in NIVEAUX_DORI) + "   points"
    print(entete)
    for li in lignes:
        cout = "?" if li.cout_eur is None else f"{li.cout_eur:.2f} €"
        stats = li.analyse.statistiques
        taux = "".join(f"{100 * stats.fraction(k):12.1f}%" for k in range(1, len(NIVEAUX_DORI) + 1))
        ok = sum(v.ok for v in li.analyse.verdicts)
        print(f"{catalogue[li.cle].nom:<34}{cout:>9}{taux}   {ok}/{len(li.analyse.verdicts)}")

    if args.csv:
        with _sortie(args.csv, args.csv).open("w", newline="", encoding="utf-8") as f:
            w = csv.writer(f)
            w.writerow(["modele", "cout_eur", "surface_m2"] + [f"{n[0]}_m2" for n in NIVEAUX_DORI]
                       + ["redondance_m2"] + [f"point_{v.nom}" for v in lignes[0].analyse.verdicts])
            for li in lignes:
                stats = li.analyse.statistiques
                w.writerow([li.cle, li.cout_eur, round(stats.surface_m2, 2)]
                           + [round(a, 2) for a in stats.au_moins_m2] + [round(stats.redondance_m2, 2)]
                           + [libelle_niveau(v.meilleur) for v in li.analyse.verdicts])
        print(f"Tableau : {args.csv}")

    ncol = min(3, len(lignes))
    nlig = math.ceil(len(lignes) / ncol)
    fig, axes = plt.subplots(nlig, ncol, figsize=(6.2 * ncol, 4.6 * nlig + 0.8), squeeze=False)
    for ax, li in zip(axes.flat, lignes):
        m = catalogue[li.cle]
        cout = "" if li.cout_eur is None else f", {li.cout_eur:.0f} €"
        stats = li.analyse.statistiques
        dessiner_carte(ax, li.plan, li.analyse.couverture, catalogue, li.analyse.verdicts,
                       theme=args.theme,
                       titre=f"{m.nom}{cout}\nIdentifier {100 * stats.fraction(4):.0f} %, "
                             f"Reconnaître {100 * stats.fraction(3):.0f} %"
                             + ("" if m.verifie else "  [fiche non vérifiée]"),
                       avec_legende=False)
    for ax in list(axes.flat)[len(lignes):]:
        ax.set_axis_off()
    leg = legende(axes.flat[0], args.theme, args.nuit)
    leg.remove()
    fig.legend(handles=leg.legend_handles, loc="lower center", ncol=6, fontsize=8, frameon=False,
               labelcolor=THEMES[args.theme]["texte"])
    fig.set_facecolor(THEMES[args.theme]["fond"])
    fig.tight_layout(rect=(0, 0.05, 1, 1))
    sortie = _sortie(args.sortie, f"sorties/comparaison_{plan.nom}{'_nuit' if args.nuit else ''}.png")
    fig.savefig(sortie, dpi=args.dpi, facecolor=fig.get_facecolor())
    plt.close(fig)
    print(f"Planche : {sortie}")
    return 0


def cmd_fenetre(args):
    from ..gui.fenetre import lancer

    plan, catalogue = _charger(args)
    lancer(plan, catalogue, args.plan, pas=args.pas or 0.2)
    return 0


def cmd_placer(args):
    plan, catalogue = _charger(args)
    p = placer(plan, catalogue, nb_cameras=args.cameras, modeles=args.modeles,
               pas_grille=args.pas_grille, pas_azimut=args.pas_azimut,
               hauteurs=args.hauteurs, inclinaisons=args.inclinaisons,
               budget_eur=args.budget, gain_minimal_m2=args.gain_minimal)
    print(f"{p['essais']} poses essayées, {p['candidats']} candidats conformes au droit "
          f"(une pose avec un modèle)")
    if not p["poses"]:
        print("Aucune pose conforme : baissez les caméras, inclinez-les davantage, "
              "ou rapprochez-les de la clôture")
        return 1
    for pose, etape in zip(p["poses"], p["etapes"]):
        part = 100 * etape["cumul_m2"] / p["surface_m2"] if p["surface_m2"] else 0.0
        print(f"  {pose.nom:<5} ({pose.x:6.2f}, {pose.y:6.2f})  {pose.hauteur:.1f} m  "
              f"az {pose.azimut:3.0f}°  plongée {pose.inclinaison:2.0f}°  "
              f"{catalogue[pose.modele].nom}")
        print(f"        apporte {etape['gain_m2']:5.1f} m², cumul {etape['cumul_m2']:5.1f} m² "
              f"({part:.0f} %), dépense {etape['cout_eur']:.0f} €")
    part = 100 * p["couverture_m2"] / p["surface_m2"] if p["surface_m2"] else 0.0
    print(f"Retenu : {len(p['poses'])} caméras, {p['cout_eur']:.0f} €, "
          f"{p['couverture_m2']:.0f} m² sur {p['surface_m2']:.0f} ({part:.0f} %), "
          f"points tenus {p['points_tenus']}/{len(plan.points)}")
    if args.sortie:
        propose = plan.copie()
        propose.cameras = p["poses"]
        enregistrer_plan(propose, args.sortie)
        print(f"Plan proposé : {args.sortie}")
    return 0


def cmd_cotes(args):
    plan, catalogue = _charger(args)
    print(f"Plan '{plan.nom}', statut : {plan.statut}")
    print("Cotes à relever sur le terrain (x vers l'est, y vers le nord, en mètres) :\n")
    print(f"- limite du terrain : {len(plan.limite)} sommets")
    for ob in plan.obstacles:
        tranche = f"{ob.base:g} à {ob.hauteur:g} m" if ob.base else f"{ob.hauteur:g} m"
        print(f"- {ob.type} '{ob.nom}' : {len(ob.points)} sommets et sa hauteur "
              f"(actuellement {tranche})")
    for c in plan.cameras:
        print(f"- caméra '{c.nom}' : position de fixation, hauteur de fixation, "
              f"azimut visé, inclinaison (actuellement {c.hauteur:g} m, {c.azimut:g}°, {c.inclinaison:g}°)")
    for p in plan.points:
        print(f"- point de passage '{p.nom}' : position, niveau requis {libelle_niveau(p.requis)}")
    print(f"- hauteur de la cible jugée (visage) : {plan.hauteur_cible:g} m")
    print("\nObstacles à ne pas oublier : arbres et massifs persistants, voitures garées "
          "habituellement, poteaux, auvents. Une fois relevé, passez statut à releve")
    return 0


def cmd_catalogue(args):
    for m in charger_catalogue(args.catalogue).values():
        print(f"{m.cle:<22}{texte_modele(m)}")
    return 0


def construire_parseur():
    p = argparse.ArgumentParser(prog="champ_vision",
                                description="Champ de vision et zones DORI des caméras sur un plan")
    p.add_argument("--catalogue", default=CATALOGUE_DEFAUT, help="catalogue YAML des caméras")
    sous = p.add_subparsers(dest="commande", required=True)

    def commun(sp, graphique=True):
        sp.add_argument("plan", nargs="?", default=PLAN_EXEMPLE, help="plan YAML (défaut : exemple)")
        sp.add_argument("--nuit", action="store_true", help="limiter chaque caméra à sa portée IR")
        if graphique:
            sp.add_argument("--pas", type=float, default=None, help="pas de la grille en mètres")
            sp.add_argument("-o", "--sortie", default=None, help="fichier image ou PDF produit")
            sp.add_argument("--theme", choices=("document", "ecran"), default="document")
            sp.add_argument("--dpi", type=int, default=160)

    sp = sous.add_parser("carte", help="carte des zones DORI")
    commun(sp)
    sp.add_argument("--hauteur-cible", type=float, default=None, help="hauteur du point jugé, en mètres")
    sp.set_defaults(func=cmd_carte)

    sp = sous.add_parser("comparer", help="même implantation, un modèle après l'autre")
    commun(sp)
    sp.add_argument("--modeles", nargs="+", default=None, help="clés du catalogue (défaut : toutes)")
    sp.add_argument("--cameras", nargs="+", default=None, help="caméras à remplacer (défaut : toutes)")
    sp.add_argument("--csv", default=None, help="tableau des résultats")
    sp.set_defaults(func=cmd_comparer)

    sp = sous.add_parser("verifier", help="niveau DORI atteint sur chaque point de passage")
    commun(sp, graphique=False)  
    sp.set_defaults(func=cmd_verifier)

    sp = sous.add_parser("fenetre", help="placement interactif des caméras")
    commun(sp, graphique=False)
    sp.add_argument("--pas", type=float, default=0.2, help="pas de la grille en mètres")
    sp.set_defaults(func=cmd_fenetre)

    sp = sous.add_parser("placer", help="propose des poses conformes au droit")
    commun(sp, graphique=False)
    sp.add_argument("--cameras", type=int, default=None,
                    help="nombre imposé, sinon le logiciel s'arrête quand une de plus ne paie plus")
    sp.add_argument("--modeles", nargs="+", default=None,
                    help="modèles en concurrence, sinon tout le catalogue")
    sp.add_argument("--budget", type=float, default=None, help="plafond de dépense en euros")
    sp.add_argument("--gain-minimal", type=float, default=5.0,
                    help="surface qu'une caméra de plus doit apporter, en mètres carrés")
    sp.add_argument("--pas-grille", type=float, default=0.3, help="pas de la grille de recherche")
    sp.add_argument("--pas-azimut", type=float, default=10.0, help="pas angulaire balayé")
    sp.add_argument("--hauteurs", type=float, nargs="+", default=None,
                    help="hauteurs de fixation essayées, en mètres")
    sp.add_argument("--inclinaisons", type=float, nargs="+", default=None,
                    help="plongées essayées, en degrés")
    sp.add_argument("-o", "--sortie", default=None, help="plan YAML à écrire avec ces poses")
    sp.set_defaults(func=cmd_placer)

    sp = sous.add_parser("cotes", help="liste des cotes à relever pour ce plan")
    commun(sp, graphique=False)
    sp.set_defaults(func=cmd_cotes)

    sp = sous.add_parser("catalogue", help="modèles disponibles et portées DORI")
    sp.set_defaults(func=cmd_catalogue)
    return p


def main(argv=None):
    args = construire_parseur().parse_args(argv)
    try:
        return args.func(args)
    except ErreurPlan as e:
        print(f"Erreur de plan : {e}", file=sys.stderr)
        return 1
