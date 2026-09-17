//! Hypothèses modifiables (« paramètres 2026 ») et types d'allocation.
//! Tous les taux sont exprimés en fraction (0,05 = 5 %).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Lignes de la poche 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Ligne {
    EtfMonde,
    FondsEurosObligations,
    Immobilier,
    Or,
    ActionsDirectes,
    Crowdfunding,
    Crypto,
    PrivateEquity,
}

impl Ligne {
    pub const TOUTES: [Ligne; 8] = [
        Ligne::EtfMonde,
        Ligne::FondsEurosObligations,
        Ligne::Immobilier,
        Ligne::Or,
        Ligne::ActionsDirectes,
        Ligne::Crowdfunding,
        Ligne::Crypto,
        Ligne::PrivateEquity,
    ];

    pub fn libelle(self) -> &'static str {
        match self {
            Ligne::EtfMonde => "ETF actions monde",
            Ligne::FondsEurosObligations => "Fonds euros / obligations",
            Ligne::Immobilier => "Immobilier (locatif net + SCPI)",
            Ligne::Or => "Or",
            Ligne::ActionsDirectes => "Actions en direct",
            Ligne::Crowdfunding => "Crowdfunding (immo + ENR)",
            Ligne::Crypto => "Crypto",
            Ligne::PrivateEquity => "Private equity",
        }
    }

    pub fn est_satellite(self) -> bool {
        matches!(
            self,
            Ligne::ActionsDirectes | Ligne::Crowdfunding | Ligne::Crypto | Ligne::PrivateEquity
        )
    }
}

/// Répartition par ligne (fractions ou montants selon le contexte).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS, Default)]
#[ts(export)]
pub struct Allocation {
    pub etf_monde: f64,
    pub fonds_euros_obligations: f64,
    pub immobilier: f64,
    pub or: f64,
    pub actions_directes: f64,
    pub crowdfunding: f64,
    pub crypto: f64,
    pub private_equity: f64,
}

impl Allocation {
    pub fn get(&self, l: Ligne) -> f64 {
        match l {
            Ligne::EtfMonde => self.etf_monde,
            Ligne::FondsEurosObligations => self.fonds_euros_obligations,
            Ligne::Immobilier => self.immobilier,
            Ligne::Or => self.or,
            Ligne::ActionsDirectes => self.actions_directes,
            Ligne::Crowdfunding => self.crowdfunding,
            Ligne::Crypto => self.crypto,
            Ligne::PrivateEquity => self.private_equity,
        }
    }

    pub fn get_mut(&mut self, l: Ligne) -> &mut f64 {
        match l {
            Ligne::EtfMonde => &mut self.etf_monde,
            Ligne::FondsEurosObligations => &mut self.fonds_euros_obligations,
            Ligne::Immobilier => &mut self.immobilier,
            Ligne::Or => &mut self.or,
            Ligne::ActionsDirectes => &mut self.actions_directes,
            Ligne::Crowdfunding => &mut self.crowdfunding,
            Ligne::Crypto => &mut self.crypto,
            Ligne::PrivateEquity => &mut self.private_equity,
        }
    }

    pub fn set(&mut self, l: Ligne, v: f64) {
        *self.get_mut(l) = v;
    }

    pub fn total(&self) -> f64 {
        Ligne::TOUTES.iter().map(|l| self.get(*l)).sum()
    }

    pub fn actions(&self) -> f64 {
        self.etf_monde + self.actions_directes
    }

    pub fn satellites(&self) -> f64 {
        Ligne::TOUTES
            .iter()
            .filter(|l| l.est_satellite())
            .map(|l| self.get(*l))
            .sum()
    }

    pub fn map(&self, f: impl Fn(Ligne, f64) -> f64) -> Allocation {
        let mut out = Allocation::default();
        for l in Ligne::TOUTES {
            out.set(l, f(l, self.get(l)));
        }
        out
    }

    /// Renormalise à 1 (si total > 0).
    pub fn normalisee(&self) -> Allocation {
        let t = self.total();
        if t <= 0.0 {
            return *self;
        }
        self.map(|_, v| v / t)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Profil {
    Prudent,
    Equilibre,
    Dynamique,
}

impl Profil {
    pub fn inferieur(self) -> Profil {
        match self {
            Profil::Dynamique => Profil::Equilibre,
            _ => Profil::Prudent,
        }
    }
    pub fn libelle(self) -> &'static str {
        match self {
            Profil::Prudent => "Prudent",
            Profil::Equilibre => "Équilibré",
            Profil::Dynamique => "Dynamique",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProfilParams {
    pub allocation: Allocation,
    /// Rendement nominal attendu long terme, avant fiscalité.
    pub rendement: f64,
    /// Perte maximale plausible sur 12 mois (valeur négative).
    pub perte_max: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PalierGlide {
    /// Le palier s'applique quand l'horizon (années) est ≥ cette valeur.
    pub horizon_min_ans: u32,
    pub part_actions: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Hypotheses {
    // --- Études ---
    pub age_debut_etudes: u32,
    pub frais_scolarite_public: f64,
    pub frais_scolarite_prive: f64,
    pub frais_scolarite_mixte: f64,
    pub cout_logement_annuel: f64,
    pub inflation_etudes: f64,
    /// Inflation appliquée au montant des projets datés.
    pub inflation_projets: f64,
    pub rendement_actions: f64,
    pub rendement_securise: f64,
    /// Paliers triés par horizon décroissant.
    pub glide_path: Vec<PalierGlide>,

    // --- Profils poche 2 ---
    pub prudent: ProfilParams,
    pub equilibre: ProfilParams,
    pub dynamique: ProfilParams,
    /// score ≤ seuil → Prudent.
    pub seuil_score_prudent: f64,
    /// score > seuil → Dynamique.
    pub seuil_score_dynamique: f64,
    pub h_fire_court: u32,
    pub h_fire_long: u32,

    // --- Précaution et dettes ---
    pub precaution_mois_min: f64,
    pub precaution_mois_defaut: f64,
    pub precaution_mois_max: f64,
    pub seuil_taux_dette: f64,
    pub endettement_max: f64,

    // --- Garde-fous ---
    pub age_regle_actions: u32,
    pub actions_max_tolerance_faible: f64,
    pub plancher_fonds_euros: f64,
    pub plancher_fonds_euros_horizon_court: f64,
    pub or_min: f64,
    pub or_max: f64,
    pub crowdfunding_max: f64,
    pub actions_directes_max: f64,
    pub satellites_max: f64,
    pub crypto_max: f64,
    pub immo_max: f64,
    pub immo_max_gestion_elevee: f64,
    pub seuil_concentration_immo: f64,
    pub tolerance_reequilibrage: f64,

    // --- Fiscalité / enveloppes (2026) ---
    pub plafond_pea: f64,
    pub pfu: f64,
    pub prelevements_sociaux: f64,
    pub abattement_av_seul: f64,
    pub abattement_av_couple: f64,
    pub abattement_donation: f64,
    pub plafond_per_revenus: f64,

    // --- Phase revenus passifs ---
    pub annees_derisquage: u32,
    pub annees_coussin: u32,
}

impl Default for Hypotheses {
    fn default() -> Self {
        let alloc = |etf, fe, immo, or, ad, cf| Allocation {
            etf_monde: etf,
            fonds_euros_obligations: fe,
            immobilier: immo,
            or,
            actions_directes: ad,
            crowdfunding: cf,
            crypto: 0.0,
            private_equity: 0.0,
        };
        Hypotheses {
            age_debut_etudes: 18,
            frais_scolarite_public: 3_000.0,
            frais_scolarite_prive: 10_000.0,
            frais_scolarite_mixte: 6_500.0,
            cout_logement_annuel: 9_000.0,
            inflation_etudes: 0.03,
            inflation_projets: 0.02,
            rendement_actions: 0.065,
            rendement_securise: 0.025,
            glide_path: vec![
                PalierGlide { horizon_min_ans: 13, part_actions: 0.80 },
                PalierGlide { horizon_min_ans: 8, part_actions: 0.60 },
                PalierGlide { horizon_min_ans: 5, part_actions: 0.40 },
                PalierGlide { horizon_min_ans: 3, part_actions: 0.20 },
                PalierGlide { horizon_min_ans: 0, part_actions: 0.00 },
            ],
            prudent: ProfilParams {
                allocation: alloc(0.30, 0.30, 0.25, 0.10, 0.00, 0.05),
                rendement: 0.04,
                perte_max: -0.15,
            },
            equilibre: ProfilParams {
                allocation: alloc(0.45, 0.15, 0.25, 0.05, 0.05, 0.05),
                rendement: 0.055,
                perte_max: -0.25,
            },
            dynamique: ProfilParams {
                allocation: alloc(0.55, 0.10, 0.20, 0.05, 0.05, 0.05),
                rendement: 0.065,
                perte_max: -0.35,
            },
            seuil_score_prudent: 3.0,
            seuil_score_dynamique: 5.0,
            h_fire_court: 7,
            h_fire_long: 15,
            precaution_mois_min: 3.0,
            precaution_mois_defaut: 4.0,
            precaution_mois_max: 6.0,
            seuil_taux_dette: 0.05,
            endettement_max: 0.35,
            age_regle_actions: 110,
            actions_max_tolerance_faible: 0.65,
            plancher_fonds_euros: 0.10,
            plancher_fonds_euros_horizon_court: 0.25,
            or_min: 0.05,
            or_max: 0.10,
            crowdfunding_max: 0.05,
            actions_directes_max: 0.10,
            satellites_max: 0.15,
            crypto_max: 0.03,
            immo_max: 0.40,
            immo_max_gestion_elevee: 0.50,
            seuil_concentration_immo: 0.40,
            tolerance_reequilibrage: 0.05,
            plafond_pea: 150_000.0,
            pfu: 0.314,
            prelevements_sociaux: 0.186,
            abattement_av_seul: 4_600.0,
            abattement_av_couple: 9_200.0,
            abattement_donation: 100_000.0,
            plafond_per_revenus: 0.10,
            annees_derisquage: 3,
            annees_coussin: 2,
        }
    }
}

impl Hypotheses {
    pub fn profil(&self, p: Profil) -> &ProfilParams {
        match p {
            Profil::Prudent => &self.prudent,
            Profil::Equilibre => &self.equilibre,
            Profil::Dynamique => &self.dynamique,
        }
    }

    /// Part actions du glide path pour un horizon donné.
    pub fn part_actions_glide(&self, horizon: u32) -> f64 {
        self.palier_glide(horizon).map(|p| p.part_actions).unwrap_or(0.0)
    }

    pub fn palier_glide(&self, horizon: u32) -> Option<&PalierGlide> {
        let mut paliers: Vec<&PalierGlide> = self.glide_path.iter().collect();
        paliers.sort_by_key(|p| std::cmp::Reverse(p.horizon_min_ans));
        paliers.into_iter().find(|p| horizon >= p.horizon_min_ans)
    }
}
