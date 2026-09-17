//! Structure de sortie (section 7 de la spécification, enrichie).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::model::{PosteDepense, Priorite, TempsGestion};
use crate::params::{Allocation, Ligne, Profil};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Gravite {
    Info,
    Attention,
    Critique,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Alerte {
    pub gravite: Gravite,
    pub message: String,
}

impl Alerte {
    pub fn info(m: impl Into<String>) -> Self {
        Alerte { gravite: Gravite::Info, message: m.into() }
    }
    pub fn attention(m: impl Into<String>) -> Self {
        Alerte { gravite: Gravite::Attention, message: m.into() }
    }
    pub fn critique(m: impl Into<String>) -> Self {
        Alerte { gravite: Gravite::Critique, message: m.into() }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Poche0 {
    pub cible_mois: f64,
    pub cible_eur: f64,
    pub existant_eur: f64,
    /// Montant manquant (> 0) pour atteindre la cible.
    pub manque_eur: f64,
    /// Montant au-delà de la cible, redéployable en poche 2.
    pub excedent_eur: f64,
    pub supports: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AllocationEtudes {
    pub etf_monde: f64,
    pub fonds_euros: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum CategoriePoche {
    Etudes,
    Projet,
}

/// Sous-poche datée de la poche 1 : études d'un enfant ou projet du foyer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SousPoche {
    pub nom: String,
    pub categorie: CategoriePoche,
    pub priorite: Priorite,
    pub horizon_ans: u32,
    /// Coût total en euros d'aujourd'hui.
    pub montant_actuel_eur: f64,
    /// Études uniquement : coût annuel en euros d'aujourd'hui.
    pub cout_annuel_actuel_eur: Option<f64>,
    pub capital_cible_eur: f64,
    pub capital_actuel_eur: f64,
    pub epargne_mensuelle_eur: f64,
    /// Si l'horizon est nul : montant manquant immédiatement.
    pub deficit_immediat_eur: f64,
    pub rendement_attendu: f64,
    pub allocation: AllocationEtudes,
    /// Années avant le prochain palier du glide path (None si déjà 100 % sécurisé).
    pub prochain_palier_dans_ans: Option<u32>,
    pub prochaine_part_actions: Option<f64>,
    pub enveloppe: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DetailScore {
    pub libelle: String,
    pub points: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LigneAllocation {
    pub ligne: Ligne,
    pub libelle: String,
    pub cible_pct: f64,
    pub actuelle_pct: f64,
    pub ecart_points: f64,
    pub actuel_eur: f64,
    pub cible_eur: f64,
    /// Répartition proposée des liquidités à déployer (stock).
    pub deploiement_eur: f64,
    /// Répartition du flux mensuel poche 2.
    pub flux_mensuel_eur: f64,
    /// Ligne fermée aux nouveaux apports.
    pub gelee: bool,
    pub enveloppe: String,
    pub support: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Levier {
    pub titre: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PointProjection {
    pub annee: u32,
    pub capital_flux_disponible: f64,
    pub capital_epargne_necessaire: f64,
    pub cible: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Poche2 {
    pub profil: Profil,
    pub profil_libelle: String,
    pub score: f64,
    pub detail_score: Vec<DetailScore>,
    pub rendement_attendu: f64,
    pub perte_max_plausible: f64,
    pub revenu_manquant_mensuel: f64,
    pub capital_cible_eur: f64,
    /// Capital financier actuel pris en compte (hors locatif direct).
    pub capital_financier_actuel_eur: f64,
    /// Base des pourcentages : financier poche 2 + immobilier locatif net.
    pub base_allocation_eur: f64,
    pub epargne_mensuelle_necessaire_eur: f64,
    /// Flux actuel vers la poche 2 une fois les projets en cours financés.
    pub flux_disponible_regime_eur: f64,
    /// Flux moyen vers la poche 2 d'ici l'échéance (projets terminés = flux libéré).
    pub flux_moyen_simule_eur: f64,
    /// Capital simulé à l'échéance avec l'épargne du foyer.
    pub capital_projete_eur: f64,
    pub atteignable: bool,
    pub allocation_cible: Allocation,
    pub allocation_actuelle: Allocation,
    pub lignes: Vec<LigneAllocation>,
    pub ajustements: Vec<String>,
    pub leviers: Vec<Levier>,
    pub projection: Vec<PointProjection>,
    pub reequilibrage_necessaire: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FluxProjet {
    pub nom: String,
    pub priorite: Priorite,
    pub besoin_eur: f64,
    pub montant_eur: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RepartitionFlux {
    pub total_eur: f64,
    pub precaution_eur: f64,
    pub remboursement_dettes_eur: f64,
    pub projets: Vec<FluxProjet>,
    pub poche_2_eur: f64,
    pub etapes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PhaseRevenus {
    pub annee_derisquage: u32,
    pub profil_phase: Profil,
    pub retrait_annuel_eur: f64,
    pub retrait_mensuel_eur: f64,
    pub coussin_fonds_euros_eur: f64,
    pub abattement_av_eur: f64,
    pub ordre_sources: Vec<String>,
    pub regles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RevenuAdulte {
    pub prenom: String,
    pub montant_eur: f64,
    pub part: f64,
    pub tol_risque: u8,
    pub stab_revenus: u8,
    pub temps_gestion: TempsGestion,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PosteBudget {
    pub poste: PosteDepense,
    pub libelle: String,
    pub montant_eur: f64,
    /// Part des revenus du foyer.
    pub part_revenus: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Budget {
    pub adultes: Vec<RevenuAdulte>,
    pub total_revenus_eur: f64,
    /// Dépenses regroupées par poste (postes vides omis).
    pub postes: Vec<PosteBudget>,
    pub total_depenses_eur: f64,
    pub mensualites_credits_eur: f64,
    pub capacite_calculee_eur: f64,
    pub epargne_retenue_eur: f64,
    pub epargne_forcee: bool,
    /// Épargne retenue / revenus.
    pub taux_epargne: f64,
    /// Capacité calculée non épargnée (si l'épargne imposée est plus faible).
    pub non_affecte_eur: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Synthese {
    pub nb_adultes: u32,
    pub nb_enfants: u32,
    pub age_reference: u32,
    pub tol_risque_retenue: u8,
    pub p_fin_eur: f64,
    pub v_immo_loc_eur: f64,
    pub ratio_immo_locatif: f64,
    pub taux_endettement: f64,
    pub tmi_pct: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Resultat {
    pub synthese: Synthese,
    pub budget: Budget,
    pub poche_0: Poche0,
    pub poche_1: Vec<SousPoche>,
    pub poche_2: Poche2,
    pub flux: RepartitionFlux,
    pub priorite_enveloppes: Vec<String>,
    pub phase_revenus: PhaseRevenus,
    pub alertes: Vec<Alerte>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ErreursValidation {
    pub erreurs: Vec<String>,
}
