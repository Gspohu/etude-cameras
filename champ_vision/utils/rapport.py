#!/usr/bin/env python3
# -*- coding: utf-8 -*-

"""Plain text summaries shared by the command line and the interactive window"""

from ..moteur import NIVEAUX_DORI, libelle_niveau


def texte_statistiques(stats):
    lignes = [f"Surface surveillée : {stats.surface_m2:.0f} m²"]
    for k, (_, nom, _) in enumerate(NIVEAUX_DORI, start=1):
        lignes.append(f"  {nom:<12} {stats.au_moins_m2[k - 1]:7.1f} m²  {100 * stats.fraction(k):5.1f} %")
    part = stats.redondance_m2 / stats.surface_m2 if stats.surface_m2 else 0.0
    lignes.append(f"  vu par 2 caméras ou plus : {stats.redondance_m2:.1f} m²  {100 * part:.1f} %")
    return "\n".join(lignes)


def texte_verdicts(verdicts, detail=True):
    lignes = []
    for v in verdicts:
        etat = "OK " if v.ok else "NON"   
        lignes.append(f"[{etat}] {v.nom} : {libelle_niveau(v.meilleur)}, requis {libelle_niveau(v.requis)}")
        if v.conseil:
            lignes.append(f"      {v.conseil}")
        if not detail:
            continue
        for d in v.diagnostics:
            if d.raison:
                lignes.append(f"      {d.camera} : {d.raison} ({d.distance:.1f} m)")
            else:
                lignes.append(f"      {d.camera} : {d.rho:.0f} px/m à {d.distance:.1f} m, "
                              f"{libelle_niveau(d.niveau)}")
    return "\n".join(lignes)


def texte_conformite(liste):
    lignes = ["Hors de la parcelle, ce que chaque caméra attrape :"]
    for c in liste:
        etat = "conforme" if c.conforme else "NON CONFORME"
        lignes.append(f"  {c.camera:<12} {c.debordement_m2:7.1f} m²  {etat}")
    if any(not c.conforme for c in liste):
        lignes.append("  Une caméra privée ne filme ni la voie publique ni le terrain voisin")
    return "\n".join(lignes)


def texte_modele(m):
    portees = ", ".join(f"{n[1]} {p:.1f} m" for n, p in zip(NIVEAUX_DORI, m.portees))
    ir = "IR inconnue" if m.portee_ir_m is None else f"IR {m.portee_ir_m:g} m"
    alerte = "" if m.verifie else "  [fiche NON vérifiée]"
    return f"{m.nom} : {m.hfov_deg:g} x {m.vfov_deg:g}°, {m.largeur_px} px, {ir}, {portees}{alerte}"
