#!/usr/bin/env python3
# -*- coding: utf-8 -*-

"""Binding to the Rust engine : data containers and calls, no physics lives here"""

import copy
import json
from dataclasses import dataclass, field
from pathlib import Path

import champ_rs
import numpy as np

DOSSIER_DONNEES = Path(__file__).resolve().parent / "data"
CATALOGUE_DEFAUT = DOSSIER_DONNEES / "cameras.yaml"
PLAN_EXEMPLE = DOSSIER_DONNEES / "plan_exemple.yaml"

NIVEAUX_DORI = tuple(champ_rs.niveaux_dori())
SEUILS_DORI = np.array([n[2] for n in NIVEAUX_DORI])


class ErreurPlan(ValueError):
    pass


def index_niveau(cle):
    for i, (c, _, _) in enumerate(NIVEAUX_DORI):
        if (c == cle):
            return i + 1
    attendus = ", ".join(n[0] for n in NIVEAUX_DORI)
    raise ErreurPlan(f"niveau DORI inconnu : {cle!r}, valeurs admises : {attendus}")


def libelle_niveau(index):
    if (index <= 0):
        return "rien"
    return NIVEAUX_DORI[index - 1][1]


@dataclass(frozen=True)
class ModeleCamera:
    cle: str
    nom: str
    focale_mm: float | None
    largeur_px: int
    hauteur_px: int
    hfov_deg: float
    vfov_deg: float
    portee_ir_m: float | None
    prix_eur: float | None
    verifie: bool
    source: str
    f_px: float
    portees: tuple

    def portee(self, seuil_px_m):
        return self.f_px / seuil_px_m

    def densite(self, distance):
        return self.f_px / max(distance, 1e-6)


@dataclass
class Pose:
    nom: str
    modele: str
    x: float
    y: float
    hauteur: float
    azimut: float
    inclinaison: float = 0.0


@dataclass
class Obstacle:
    nom: str
    type: str
    points: list
    hauteur: float
    ferme: bool 
    opaque: bool
    base: float = 0.0

    @property
    def exclu_surface(self):
        return self.type == "batiment" and self.ferme


@dataclass
class PointPassage:
    nom: str
    x: float
    y: float
    requis: int
    hauteur: float | None = None


@dataclass
class Plan:
    nom: str
    statut: str
    limite: list
    obstacles: list = field(default_factory=list)
    cameras: list = field(default_factory=list)
    points: list = field(default_factory=list)
    hauteur_cible: float = 1.6
    pas: float = 0.1
    # width of the band looked at past the boundary, it sets what counts as filmed outside
    marge_hors: float = 12.0

    @property
    def cotes_arbitraires(self):
        return self.statut != "releve"

    def camera(self, nom):
        for p in self.cameras:
            if (p.nom == nom):
                return p
        raise KeyError(f"aucune caméra nommée {nom!r} dans le plan, présentes : "
                       + ", ".join(p.nom for p in self.cameras))

    def copie(self):
        return copy.deepcopy(self)


@dataclass
class Statistiques:
    surface_m2: float
    au_moins_m2: list
    redondance_m2: float

    def fraction(self, k):
        return self.au_moins_m2[k - 1] / self.surface_m2 if self.surface_m2 else 0.0


@dataclass
class DiagnosticCamera:
    camera: str
    raison: str | None
    distance: float
    rho: float
    niveau: int


@dataclass
class VerdictPoint:
    nom: str
    requis: int
    diagnostics: list
    portee_requise: float
    meilleur: int
    ok: bool
    conseil: str | None


@dataclass
class Conformite:
    camera: str
    debordement_m2: float
    conforme: bool


@dataclass
class Couverture:
    x0: float
    y0: float
    pas: float
    nuit: bool
    hauteur_cible: float
    mention: str | None
    utile: np.ndarray
    dehors: np.ndarray
    debordement: np.ndarray
    rho: np.ndarray
    niveau: np.ndarray
    nb_cameras: np.ndarray

    @property
    def xs(self):
        return self.x0 + self.pas * np.arange(self.utile.shape[1])

    @property
    def ys(self):
        return self.y0 + (self.pas * np.arange(self.utile.shape[0]))

    @property
    def rho_max(self):
        if not len(self.rho):
            return np.zeros(self.utile.shape)
        return self.rho.max(axis=0)

    @property
    def etendue(self):
        xs, ys = self.xs, self.ys
        return (xs[0] - (self.pas / 2), xs[-1] + self.pas / 2,
                ys[0] - self.pas / 2, ys[-1] + self.pas / 2)


@dataclass
class Analyse:
    couverture: Couverture
    statistiques: Statistiques
    verdicts: list
    conformite: list


def _modele(cle, b):
    return ModeleCamera(cle=cle, nom=b["nom"], focale_mm=b.get("focale_mm"),
                        largeur_px=b["largeur_px"], hauteur_px=b["hauteur_px"],
                        hfov_deg=b["hfov"], vfov_deg=b["vfov"], portee_ir_m=b.get("portee_ir_m"),
                        prix_eur=b.get("prix_eur"), verifie=b["verifie"], source=b["source"],
                        f_px=b["f_px"], portees=tuple(b["portees"]))


def charger_catalogue(chemin=CATALOGUE_DEFAUT):
    try:
        brut = champ_rs.charger_catalogue(Path(chemin).read_text(encoding="utf-8"))
    except (ValueError, OSError) as e:
        raise ErreurPlan(str(e)) from None
    return {cle: _modele(cle, b) for cle, b in sorted(json.loads(brut).items())}


def charger_plan(chemin, catalogue):   
    chemin = Path(chemin)
    try:
        brut = champ_rs.charger_plan(chemin.read_text(encoding="utf-8"),
                                     _catalogue_json(catalogue), chemin.stem)
    except (ValueError, OSError) as e:
        raise ErreurPlan(str(e)) from None
    d = json.loads(brut)
    plan = Plan(nom=d["nom"], statut=d["statut"], limite=[tuple(p) for p in d["limite"]],
                hauteur_cible=d["hauteur_cible"], pas=d["pas"], marge_hors=d["marge_hors"])
    plan.obstacles = [Obstacle(nom=o["nom"], type=o["type"], points=[tuple(p) for p in o["points"]],
                               hauteur=o["hauteur"], ferme=o["ferme"], opaque=o["opaque"],
                               base=o["base"]) for o in d["obstacles"]]
    plan.cameras = [Pose(**c) for c in d["cameras"]]
    plan.points = [PointPassage(**p) for p in d["points"]]
    return plan


def enregistrer_plan(plan, chemin):
    texte = champ_rs.plan_vers_yaml(json.dumps(_plan_dict(plan)))
    Path(chemin).write_text(texte, encoding="utf-8")


def _plan_dict(plan):
    return {
        "nom": plan.nom, "statut": plan.statut, "limite": [list(p) for p in plan.limite],
        "hauteur_cible": plan.hauteur_cible, "pas": plan.pas, "marge_hors": plan.marge_hors,
        "obstacles": [{"nom": o.nom, "type": o.type, "points": [list(p) for p in o.points],
                       "hauteur": o.hauteur, "base": o.base, "ferme": o.ferme,
                       "opaque": o.opaque} for o in plan.obstacles],
        "cameras": [{"nom": c.nom, "modele": c.modele, "x": c.x, "y": c.y, "hauteur": c.hauteur,
                     "azimut": c.azimut, "inclinaison": c.inclinaison} for c in plan.cameras],
        "points": [{"nom": p.nom, "x": p.x, "y": p.y, "requis": p.requis,
                    "hauteur": p.hauteur} for p in plan.points],
    }


def _catalogue_json(catalogue):
    return json.dumps({cle: {"cle": m.cle, "nom": m.nom, "focale_mm": m.focale_mm,
                             "largeur_px": m.largeur_px, "hauteur_px": m.hauteur_px,
                             "hfov": m.hfov_deg, "vfov": m.vfov_deg,
                             "portee_ir_m": m.portee_ir_m, "prix_eur": m.prix_eur,
                             "verifie": m.verifie, "source": m.source}
                       for cle, m in catalogue.items()})


def _requete(plan, catalogue, pas=None, nuit=False, hauteur_cible=None):
    return json.dumps({"plan": _plan_dict(plan), "catalogue": json.loads(_catalogue_json(catalogue)),
                       "pas": pas, "nuit": nuit, "hauteur_cible": hauteur_cible})


def _verdicts(brut):
    sortie = []
    for v in brut:
        diags = [DiagnosticCamera(**d) for d in v["diagnostics"]]
        sortie.append(VerdictPoint(nom=v["nom"], requis=v["requis"], diagnostics=diags,
                                   portee_requise=v["portee_requise"], meilleur=v["meilleur"],
                                   ok=v["ok"], conseil=v["conseil"]))
    return sortie


def analyser(plan, catalogue, pas=None, nuit=False, hauteur_cible=None):
    try:
        a = champ_rs.analyser(_requete(plan, catalogue, pas, nuit, hauteur_cible))
    except ValueError as e:
        raise ErreurPlan(str(e)) from None
    r = json.loads(a.resume)
    forme = (r["ny"], r["nx"])
    utile = np.frombuffer(a.utile(), dtype=np.uint8).reshape(forme).astype(bool)
    rho = np.frombuffer(a.rho(), dtype="<f8").reshape((len(plan.cameras),) + forme)
    couv = Couverture(x0=r["x0"], y0=r["y0"], pas=r["pas"], nuit=r["nuit"],
                      hauteur_cible=r["hauteur_cible"], mention=r["mention"], utile=utile,
                      dehors=np.frombuffer(a.dehors(), dtype=np.uint8).reshape(forme).astype(bool),
                      debordement=np.frombuffer(a.debordement(),
                                                dtype=np.uint8).reshape(forme).astype(bool),
                      rho=rho,
                      niveau=np.frombuffer(a.niveau(), dtype=np.uint8).reshape(forme),
                      nb_cameras=np.frombuffer(a.nb_cameras(), dtype=np.uint8).reshape(forme))
    stats = Statistiques(**r["statistiques"])
    return Analyse(couv, stats, _verdicts(r["verdicts"]),
                   [Conformite(**c) for c in r["conformite"]])


def placer(plan, catalogue, nb_cameras=None, modeles=None, pas_grille=0.3,
           pas_azimut=10.0, hauteurs=None, inclinaisons=None, budget_eur=None,
           gain_minimal_m2=5.0):
    """Poses that watch the plot without filming past its boundary"""
    requete = json.loads(_requete(plan, catalogue))
    requete.update({"pas_grille": pas_grille, "pas_azimut": pas_azimut,
                    "gain_minimal_m2": gain_minimal_m2})
    if nb_cameras:
        requete["nb_cameras"] = nb_cameras
    if budget_eur:
        requete["budget_eur"] = budget_eur
    if modeles:
        requete["modeles"] = list(modeles)
    if hauteurs:
        requete["hauteurs"] = list(hauteurs)
    if inclinaisons:
        requete["inclinaisons"] = list(inclinaisons)
    try:
        p = json.loads(champ_rs.placer(json.dumps(requete)))
    except ValueError as e:
        raise ErreurPlan(str(e)) from None
    p["poses"] = [Pose(**q) for q in p["poses"]]
    return p


def verifier_points(plan, catalogue, nuit=False):
    try:
        brut = champ_rs.verifier(_requete(plan, catalogue, nuit=nuit))
    except ValueError as e:
        raise ErreurPlan(str(e)) from None
    return _verdicts(json.loads(brut))


def diagnostiquer(plan, catalogue, camera, x, y, hauteur=None, nuit=False):
    requete = json.dumps({"plan": _plan_dict(plan),
                          "catalogue": json.loads(_catalogue_json(catalogue)),
                          "camera": camera, "x": x, "y": y, "hauteur": hauteur, "nuit": nuit})
    try:
        d = json.loads(champ_rs.diagnostiquer(requete))
    except ValueError as e:
        raise ErreurPlan(str(e)) from None
    return DiagnosticCamera(camera=camera, **d)


def remplacer_modele(plan, cle, cameras=None):
    """Copy of the plan where the named cameras (all by default) carry another model"""
    nouveau = plan.copie()
    for p in nouveau.cameras:
        if (cameras is None or p.nom in cameras):
            p.modele = cle
    return nouveau


@dataclass
class LigneComparaison:
    cle: str
    plan: Plan
    analyse: Analyse
    cout_eur: float | None 


def comparer(plan, catalogue, cles, cameras=None, pas=None, nuit=False):
    lignes = []
    for cle in cles:
        variante = remplacer_modele(plan, cle, cameras)
        prix = [catalogue[p.modele].prix_eur for p in variante.cameras]
        cout = None if any(v is None for v in prix) else float(sum(prix))
        lignes.append(LigneComparaison(cle, variante, analyser(variante, catalogue, pas, nuit), cout))
    return lignes
