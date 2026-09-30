//! Variables d'entrée (questionnaire du foyer) — section 1 de la spécification,
//! étendue à la saisie « en famille » : plusieurs adultes, budget par poste,
//! projets datés.
//!
//! Conventions : montants en euros, taux saisis par l'utilisateur en pourcentage
//! (`*_pct`), durées en années entières.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::params::Hypotheses;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum TypeEcole {
    Public,
    Prive,
    Mixte,
}

/// Tranche marginale d'imposition du foyer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Tmi {
    #[serde(rename = "0")]
    T0,
    #[serde(rename = "11")]
    T11,
    #[serde(rename = "30")]
    T30,
    #[serde(rename = "41")]
    T41,
    #[serde(rename = "45")]
    T45,
}

impl Tmi {
    pub fn pct(self) -> u8 {
        match self {
            Tmi::T0 => 0,
            Tmi::T11 => 11,
            Tmi::T30 => 30,
            Tmi::T41 => 41,
            Tmi::T45 => 45,
        }
    }
    pub fn haute(self) -> bool {
        self >= Tmi::T30
    }
    pub fn basse(self) -> bool {
        self <= Tmi::T11
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum TempsGestion {
    Faible,
    Moyen,
    Eleve,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum StatutRp {
    Locataire,
    ProprietaireAvecCredit,
    /// Propriétaire, crédit remboursé ou quasi.
    ProprietairePayee,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Revenu {
    pub libelle: String,
    /// Net mensuel (moyenne lissée sur l'année pour les primes).
    pub montant_mensuel: f64,
}

/// Un adulte du foyer, avec ses revenus et son rapport au risque.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Adulte {
    pub prenom: String,
    pub age: u32,
    pub revenus: Vec<Revenu>,
    /// 1 = indépendant / variable … 3 = CDI / fonctionnaire.
    pub stab_revenus: u8,
    /// 1 = « je vends à -20 % » … 5 = « je renforce ».
    pub tol_risque: u8,
    pub temps_gestion: TempsGestion,
    pub abondement_employeur: bool,
}

impl Adulte {
    pub fn revenu_total(&self) -> f64 {
        self.revenus.iter().map(|r| r.montant_mensuel).fold(0.0, |a, b| a + b)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum PosteDepense {
    Logement,
    Alimentation,
    Transport,
    Enfants,
    SanteAssurances,
    Abonnements,
    Loisirs,
    Impots,
    Autre,
}

impl PosteDepense {
    pub const TOUS: [PosteDepense; 9] = [
        PosteDepense::Logement,
        PosteDepense::Alimentation,
        PosteDepense::Transport,
        PosteDepense::Enfants,
        PosteDepense::SanteAssurances,
        PosteDepense::Abonnements,
        PosteDepense::Loisirs,
        PosteDepense::Impots,
        PosteDepense::Autre,
    ];

    pub fn libelle(self) -> &'static str {
        match self {
            PosteDepense::Logement => "Logement (loyer, charges, énergie)",
            PosteDepense::Alimentation => "Alimentation",
            PosteDepense::Transport => "Transport",
            PosteDepense::Enfants => "Enfants (garde, cantine, activités)",
            PosteDepense::SanteAssurances => "Santé et assurances",
            PosteDepense::Abonnements => "Abonnements et télécoms",
            PosteDepense::Loisirs => "Loisirs et vacances",
            PosteDepense::Impots => "Impôts (IR, taxe foncière)",
            PosteDepense::Autre => "Autres dépenses",
        }
    }
}

/// Dépense courante mensuelle du foyer, hors mensualités de crédit (saisies dans les crédits).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Depense {
    pub poste: PosteDepense,
    pub libelle: String,
    pub montant_mensuel: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Priorite {
    Essentiel,
    Important,
    Souhaitable,
}

impl Priorite {
    pub fn libelle(self) -> &'static str {
        match self {
            Priorite::Essentiel => "Essentiel",
            Priorite::Important => "Important",
            Priorite::Souhaitable => "Souhaitable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Enfant {
    pub prenom: String,
    pub age: u32,
    pub type_ecole: TypeEcole,
    pub logement_etudiant: bool,
    /// Années à financer (prépa + école).
    pub duree_etudes: u32,
    /// Capital déjà mis de côté pour cet enfant (€).
    pub capital_deja_affecte: f64,
}

/// Projet daté du foyer (voiture, travaux, apport, voyage…).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Projet {
    pub libelle: String,
    /// Montant en euros d'aujourd'hui.
    pub montant: f64,
    /// Échéance, en années à partir d'aujourd'hui.
    pub dans_ans: u32,
    pub priorite: Priorite,
    pub capital_deja_affecte: f64,
}

/// Ce que finance le crédit : le levier n'a pas le même sens sur un bien qui rapporte
/// que sur une voiture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ObjetCredit {
    ResidencePrincipale,
    /// Immobilier locatif (en direct ou via une SCI) : le bien produit des loyers.
    Locatif,
    /// Crédit à la consommation, auto, travaux…
    Consommation,
    Autre,
}

impl ObjetCredit {
    pub fn libelle(self) -> &'static str {
        match self {
            ObjetCredit::ResidencePrincipale => "Résidence principale",
            ObjetCredit::Locatif => "Immobilier locatif",
            ObjetCredit::Consommation => "Consommation",
            ObjetCredit::Autre => "Autre",
        }
    }

    /// Un crédit adossé à un bien qui produit des revenus : le levier joue.
    pub fn productif(self) -> bool {
        self == ObjetCredit::Locatif
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Dette {
    pub libelle: String,
    pub taux_pct: f64,
    pub restant_du: f64,
    pub mensualite: f64,
    /// Nombre de mensualités restantes. 0 = durée inconnue (le crédit est alors
    /// supposé courir jusqu'au bout de la projection).
    pub duree_restante_mois: u32,
    pub objet: ObjetCredit,
    /// Identifiant de la SCI qui porte le crédit ; `None` = crédit du foyer.
    pub sci_id: Option<String>,
}

impl Dette {
    pub fn taux(&self) -> f64 {
        self.taux_pct / 100.0
    }

    /// Mois de la dernière mensualité ; `None` si la durée n'est pas renseignée.
    pub fn fin_mois(&self) -> Option<u32> {
        (self.duree_restante_mois > 0).then_some(self.duree_restante_mois)
    }

    /// Mensualité encore due au mois `m` (0 = aujourd'hui).
    pub fn mensualite_a(&self, m: u32) -> f64 {
        match self.fin_mois() {
            Some(fin) if m >= fin => 0.0,
            _ => self.mensualite,
        }
    }
}

/// Régime fiscal de la SCI. Le choix change tout : à l'IR les loyers sont imposés
/// chaque année chez les associés ; à l'IS la société amortit le bien et n'impose
/// les associés qu'au moment de la distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum RegimeSci {
    Ir,
    Is,
}

impl RegimeSci {
    pub fn libelle(self) -> &'static str {
        match self {
            RegimeSci::Ir => "SCI à l'IR (translucide)",
            RegimeSci::Is => "SCI à l'IS",
        }
    }
}

/// Société civile immobilière détenant de l'immobilier locatif et/ou des parts de SCPI.
/// Les crédits de la SCI sont saisis avec les autres crédits, rattachés par `sci_id`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Sci {
    /// Identifiant stable, utilisé par `Dette::sci_id`.
    pub id: String,
    pub nom: String,
    pub regime: RegimeSci,
    /// Quote-part détenue par le foyer (%) : une SCI est souvent partagée avec des tiers.
    pub part_foyer_pct: f64,
    /// Valeur vénale des biens détenus par la SCI.
    pub valeur_biens: f64,
    /// Base d'acquisition amortissable des biens (distincte de leur valeur vénale).
    pub base_amortissement: f64,
    /// Parts de SCPI détenues par la SCI.
    pub scpi: f64,
    pub loyers_mensuels: f64,
    /// Charges mensuelles hors crédit et hors impôt (taxe foncière, gestion, travaux courants).
    pub charges_mensuelles: f64,
    /// SCI à l'IS : part du résultat distribuée aux associés (%). Le reste est capitalisé
    /// dans la société et ne constitue donc pas un revenu passif du foyer.
    pub distribution_pct: f64,
}

impl Sci {
    pub fn part_foyer(&self) -> f64 {
        (self.part_foyer_pct / 100.0).clamp(0.0, 1.0)
    }
}

/// Bien vendable : le locatif détenu en direct, ou les biens d'une SCI.
/// La résidence principale est hors périmètre (elle changerait aussi le budget logement).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(export)]
pub enum BienVendu {
    /// Immobilier locatif détenu en direct par le foyer.
    LocatifDirect,
    /// Biens d'une SCI, désignée par son identifiant.
    Sci { id: String },
}

/// Vente d'un bien immobilier programmée à une échéance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct VenteImmobiliere {
    pub libelle: String,
    pub bien: BienVendu,
    /// Échéance de la vente, en années à partir d'aujourd'hui.
    pub dans_ans: u32,
    /// Part du bien vendue (%) : 100 = vente totale.
    pub part_vendue_pct: f64,
    /// Prix de vente attendu, en euros d'aujourd'hui.
    pub prix_vente: f64,
    /// Prix d'acquisition, frais d'acquisition et travaux inclus (base de la plus-value).
    pub prix_acquisition: f64,
    /// Années de détention déjà écoulées à ce jour (les abattements courent en plus
    /// jusqu'à la date de vente).
    pub detention_ans: u32,
    /// Frais d'agence et de diagnostics, en % du prix de vente.
    pub frais_vente_pct: f64,
}

impl VenteImmobiliere {
    pub fn part_vendue(&self) -> f64 {
        (self.part_vendue_pct / 100.0).clamp(0.0, 1.0)
    }

    /// Durée de détention au jour de la vente.
    pub fn detention_a_la_vente(&self) -> u32 {
        self.detention_ans + self.dans_ans
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, Default)]
#[ts(export)]
pub struct Contraintes {
    pub esg: bool,
    pub refus_crypto: bool,
    pub refus_immo_direct: bool,
    /// Part de crypto souhaitée dans la poche 2 (%), plafonnée par les hypothèses.
    pub crypto_souhaitee_pct: f64,
}

/// Avoirs financiers actuels du foyer (hors résidence principale, hors capital déjà affecté
/// aux études et aux projets).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, Default)]
#[ts(export)]
pub struct Avoirs {
    /// Livret A, LDDS, LEP… (épargne de précaution).
    pub livrets: f64,
    /// Liquidités à investir (compte courant excédentaire, cash PEA/CTO).
    pub liquidites_a_investir: f64,
    pub etf_monde: f64,
    pub fonds_euros_obligations: f64,
    pub scpi: f64,
    pub or: f64,
    pub actions_directes: f64,
    pub crowdfunding_immo: f64,
    pub crowdfunding_enr: f64,
    pub crypto: f64,
    pub private_equity: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Questionnaire {
    // --- Foyer ---
    pub adultes: Vec<Adulte>,
    pub tmi: Tmi,
    pub rp: StatutRp,
    pub lep_eligible: bool,

    // --- Budget ---
    pub depenses: Vec<Depense>,
    /// Épargne mensuelle imposée ; `None` = revenus − dépenses − mensualités.
    pub epargne_mensuelle_forcee: Option<f64>,

    // --- Objectif revenus passifs ---
    /// Années avant de réduire le temps de travail.
    pub h_fire: u32,
    /// Revenu passif mensuel visé pour le foyer (net, €/mois).
    pub r_cible: f64,
    pub taux_retrait_pct: f64,
    pub autres_revenus_passifs: f64,

    // --- Enfants et projets ---
    pub enfants: Vec<Enfant>,
    pub projets: Vec<Projet>,

    // --- Patrimoine ---
    pub avoirs: Avoirs,
    /// Valeur nette (valeur − capital restant dû) de l'immobilier locatif détenu en direct.
    pub v_immo_loc: f64,
    /// Cash-flow net mensuel du locatif direct (après crédit, charges, impôts).
    pub cf_immo: f64,
    /// Loyers mensuels bruts du locatif direct, utilisés pour le ratio bancaire.
    pub loyers_immo_loc_mensuels: f64,
    /// Sociétés civiles immobilières du foyer.
    pub scis: Vec<Sci>,
    /// Ventes immobilières programmées.
    pub ventes: Vec<VenteImmobiliere>,
    pub dettes: Vec<Dette>,

    // --- Préférences ---
    pub contraintes: Contraintes,
}

/// Vue agrégée du foyer, dérivée du questionnaire.
#[derive(Debug, Clone, PartialEq)]
pub struct Foyer {
    /// Âge de référence : l'adulte le plus âgé (règle actions la plus prudente).
    pub age: u32,
    pub en_couple: bool,
    /// Tolérance retenue : la plus faible des adultes.
    pub tol_risque: u8,
    pub tol_max: u8,
    /// Stabilité moyenne pondérée par les revenus (1 à 3).
    pub stab_ponderee: f64,
    /// Temps de gestion retenu : le plus faible des adultes.
    pub temps_gestion: TempsGestion,
    pub abondement_employeur: bool,
    pub revenus: f64,
    pub depenses: f64,
    /// Mensualités des crédits portés par le foyer (hors crédits de SCI).
    pub mensualites: f64,
    /// Effort d'épargne mensuel à apporter aux SCI dont la trésorerie est négative.
    pub effort_scis: f64,
    pub capacite_calculee: f64,
    /// Épargne mensuelle retenue pour la répartition.
    pub e_mois: f64,
}

impl Questionnaire {
    /// Crédits portés par le foyer lui-même (hors SCI).
    pub fn dettes_foyer(&self) -> impl Iterator<Item = &Dette> {
        self.dettes.iter().filter(|d| d.sci_id.is_none())
    }

    /// Crédits portés par les SCI.
    pub fn dettes_scis(&self) -> impl Iterator<Item = &Dette> {
        self.dettes.iter().filter(|d| d.sci_id.is_some())
    }

    /// Bilan annuel de chaque SCI, du point de vue du foyer.
    pub fn bilans_scis(&self, h: &Hypotheses) -> Vec<crate::sci::BilanSci> {
        self.scis.iter().map(|s| crate::sci::bilan(s, &self.dettes, self.tmi, h)).collect()
    }

    pub fn foyer(&self, h: &Hypotheses) -> Foyer {
        let revenus = self.adultes.iter().map(Adulte::revenu_total).fold(0.0, |a, b| a + b);
        let depenses = self.depenses.iter().map(|d| d.montant_mensuel).fold(0.0, |a, b| a + b);
        let mensualites = self.dettes_foyer().map(|d| d.mensualite).fold(0.0, |a, b| a + b);
        // Une SCI dont la trésorerie ne couvre pas son crédit est comblée par le foyer :
        // c'est autant d'épargne en moins.
        let effort_scis = self.bilans_scis(h).iter().map(|b| b.effort_mensuel_eur).fold(0.0, |a, b| a + b);
        let capacite = revenus - depenses - mensualites - effort_scis;
        let stab_ponderee = if revenus > 0.0 {
            self.adultes.iter().map(|a| a.stab_revenus as f64 * a.revenu_total()).sum::<f64>() / revenus
        } else if self.adultes.is_empty() {
            2.0
        } else {
            self.adultes.iter().map(|a| a.stab_revenus as f64).sum::<f64>() / self.adultes.len() as f64
        };
        Foyer {
            age: self.adultes.iter().map(|a| a.age).max().unwrap_or(0),
            en_couple: self.adultes.len() >= 2,
            tol_risque: self.adultes.iter().map(|a| a.tol_risque).min().unwrap_or(3),
            tol_max: self.adultes.iter().map(|a| a.tol_risque).max().unwrap_or(3),
            stab_ponderee,
            temps_gestion: self
                .adultes
                .iter()
                .map(|a| a.temps_gestion)
                .min()
                .unwrap_or(TempsGestion::Faible),
            abondement_employeur: self.adultes.iter().any(|a| a.abondement_employeur),
            revenus,
            depenses,
            mensualites,
            effort_scis,
            capacite_calculee: capacite,
            e_mois: self.epargne_mensuelle_forcee.unwrap_or(capacite).max(0.0),
        }
    }

    /// Jeu d'exemple : couple, deux enfants, deux projets.
    pub fn exemple() -> Self {
        let dep = |poste, libelle: &str, m| Depense { poste, libelle: libelle.into(), montant_mensuel: m };
        Questionnaire {
            adultes: vec![
                Adulte {
                    prenom: "Adulte 1".into(),
                    age: 36,
                    revenus: vec![Revenu { libelle: "Salaire net".into(), montant_mensuel: 3_200.0 }],
                    stab_revenus: 3,
                    tol_risque: 4,
                    temps_gestion: TempsGestion::Moyen,
                    abondement_employeur: true,
                },
                Adulte {
                    prenom: "Adulte 2".into(),
                    age: 34,
                    revenus: vec![
                        Revenu { libelle: "Salaire net".into(), montant_mensuel: 2_300.0 },
                        Revenu { libelle: "Primes (lissées)".into(), montant_mensuel: 300.0 },
                    ],
                    stab_revenus: 2,
                    tol_risque: 3,
                    temps_gestion: TempsGestion::Faible,
                    abondement_employeur: false,
                },
            ],
            tmi: Tmi::T30,
            rp: StatutRp::ProprietaireAvecCredit,
            lep_eligible: false,
            depenses: vec![
                dep(PosteDepense::Logement, "Charges, énergie", 250.0),
                dep(PosteDepense::Alimentation, "Courses", 750.0),
                dep(PosteDepense::Transport, "Carburant, entretien", 300.0),
                dep(PosteDepense::Enfants, "Garde, cantine, activités", 350.0),
                dep(PosteDepense::SanteAssurances, "Mutuelle, assurances", 180.0),
                dep(PosteDepense::Abonnements, "Box, mobiles, streaming", 90.0),
                dep(PosteDepense::Loisirs, "Sorties, vacances", 350.0),
                dep(PosteDepense::Impots, "IR + taxe foncière", 300.0),
                dep(PosteDepense::Autre, "Divers", 100.0),
            ],
            epargne_mensuelle_forcee: None,
            h_fire: 15,
            r_cible: 1_500.0,
            taux_retrait_pct: 3.5,
            autres_revenus_passifs: 0.0,
            enfants: vec![
                Enfant {
                    prenom: "Enfant 1".into(),
                    age: 6,
                    type_ecole: TypeEcole::Mixte,
                    logement_etudiant: true,
                    duree_etudes: 5,
                    capital_deja_affecte: 5_000.0,
                },
                Enfant {
                    prenom: "Enfant 2".into(),
                    age: 3,
                    type_ecole: TypeEcole::Mixte,
                    logement_etudiant: true,
                    duree_etudes: 5,
                    capital_deja_affecte: 2_000.0,
                },
            ],
            projets: vec![
                Projet {
                    libelle: "Changer de voiture".into(),
                    montant: 12_000.0,
                    dans_ans: 4,
                    priorite: Priorite::Important,
                    capital_deja_affecte: 3_000.0,
                },
                Projet {
                    libelle: "Voyage en famille".into(),
                    montant: 6_000.0,
                    dans_ans: 2,
                    priorite: Priorite::Souhaitable,
                    capital_deja_affecte: 1_000.0,
                },
            ],
            avoirs: Avoirs {
                livrets: 20_000.0,
                liquidites_a_investir: 0.0,
                etf_monde: 40_000.0,
                fonds_euros_obligations: 15_000.0,
                scpi: 5_000.0,
                or: 3_000.0,
                actions_directes: 6_000.0,
                crowdfunding_immo: 9_000.0,
                crowdfunding_enr: 0.0,
                crypto: 0.0,
                private_equity: 0.0,
            },
            v_immo_loc: 30_000.0,
            cf_immo: 100.0,
            loyers_immo_loc_mensuels: 900.0,
            scis: vec![Sci {
                id: "sci-1".into(),
                nom: "SCI du Moulin".into(),
                regime: RegimeSci::Is,
                part_foyer_pct: 50.0,
                valeur_biens: 260_000.0,
                base_amortissement: 260_000.0,
                scpi: 0.0,
                loyers_mensuels: 1_150.0,
                charges_mensuelles: 250.0,
                distribution_pct: 0.0,
            }],
            ventes: vec![VenteImmobiliere {
                libelle: "Vendre l'appartement locatif".into(),
                bien: BienVendu::LocatifDirect,
                dans_ans: 8,
                part_vendue_pct: 100.0,
                prix_vente: 190_000.0,
                prix_acquisition: 150_000.0,
                detention_ans: 6,
                frais_vente_pct: 5.0,
            }],
            dettes: vec![
                Dette {
                    libelle: "Crédit résidence principale".into(),
                    taux_pct: 1.3,
                    restant_du: 160_000.0,
                    mensualite: 1_050.0,
                    duree_restante_mois: 168,
                    objet: ObjetCredit::ResidencePrincipale,
                    sci_id: None,
                },
                Dette {
                    libelle: "Crédit SCI du Moulin".into(),
                    taux_pct: 3.1,
                    restant_du: 175_000.0,
                    mensualite: 1_010.0,
                    duree_restante_mois: 216,
                    objet: ObjetCredit::Locatif,
                    sci_id: Some("sci-1".into()),
                },
            ],
            contraintes: Contraintes::default(),
        }
    }

    /// Patrimoine financier hors RP (inclut le capital affecté aux études et projets).
    pub fn p_fin(&self) -> f64 {
        let a = &self.avoirs;
        a.livrets
            + a.liquidites_a_investir
            + a.etf_monde
            + a.fonds_euros_obligations
            + a.scpi
            + a.or
            + a.actions_directes
            + a.crowdfunding_immo
            + a.crowdfunding_enr
            + a.crypto
            + a.private_equity
            + self.enfants.iter().map(|e| e.capital_deja_affecte).sum::<f64>()
            + self.projets.iter().map(|p| p.capital_deja_affecte).sum::<f64>()
    }

    /// Immobilier locatif net total : détention directe + quote-part des SCI.
    pub fn immo_locatif_net(&self, h: &Hypotheses) -> f64 {
        self.v_immo_loc + self.bilans_scis(h).iter().map(|b| b.valeur_nette_eur).fold(0.0, |a, b| a + b)
    }

    /// SCPI détenues en direct et via les SCI.
    pub fn scpi_total(&self, h: &Hypotheses) -> f64 {
        self.avoirs.scpi + self.bilans_scis(h).iter().map(|b| b.scpi_eur).fold(0.0, |a, b| a + b)
    }

    /// Revenu passif immobilier mensuel réellement perçu par le foyer, net d'impôt :
    /// locatif direct + ce que les SCI distribuent effectivement.
    pub fn cf_immo_total(&self, h: &Hypotheses) -> f64 {
        self.cf_immo
            + self.bilans_scis(h).iter().map(|b| b.cash_flow_foyer_mensuel_eur.max(0.0)).fold(0.0, |a, b| a + b)
    }

    /// Taux d'endettement du foyer : ses seules mensualités rapportées à ses revenus.
    pub fn taux_endettement(&self, h: &Hypotheses) -> f64 {
        let f = self.foyer(h);
        if f.revenus <= 0.0 {
            return 0.0;
        }
        f.mensualites / f.revenus
    }

    /// Taux d'endettement au sens bancaire : toutes les mensualités, y compris celles des
    /// SCI, rapportées aux revenus augmentés des loyers pondérés (les banques ne retiennent
    /// qu'une partie des loyers pour couvrir la vacance et les charges).
    pub fn taux_endettement_bancaire(&self, h: &Hypotheses) -> f64 {
        let f = self.foyer(h);
        let bilans = self.bilans_scis(h);
        let mensualites_scis: f64 = bilans.iter().map(|b| b.mensualites_mensuelles_eur).fold(0.0, |a, b| a + b);
        let loyers: f64 =
            bilans.iter().map(|b| b.loyers_annuels_eur / 12.0 * b.part_foyer).fold(0.0, |a, b| a + b);
        // Le locatif direct est saisi net de crédit : on retient son cash-flow positif.
        let revenus = f.revenus + (loyers + self.loyers_immo_loc_mensuels.max(0.0)) * h.ponderation_loyers_bancaire;
        if revenus <= 0.0 {
            return 0.0;
        }
        (f.mensualites + mensualites_scis) / revenus
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agregats_du_foyer() {
        let h = Hypotheses::default();
        let q = Questionnaire::exemple();
        let f = q.foyer(&h);
        assert_eq!(f.age, 36);
        assert!(f.en_couple);
        assert_eq!(f.tol_risque, 3);
        assert_eq!(f.tol_max, 4);
        assert_eq!(f.temps_gestion, TempsGestion::Faible);
        assert!(f.abondement_employeur);
        assert!((f.revenus - 5_800.0).abs() < 1e-9);
        assert!((f.depenses - 2_670.0).abs() < 1e-9);
        // La SCI de l'exemple est en trésorerie négative : le foyer comble la différence,
        // qui vient en déduction de la capacité d'épargne.
        assert!(f.effort_scis > 0.0);
        assert!((f.capacite_calculee - (5_800.0 - 2_670.0 - 1_050.0 - f.effort_scis)).abs() < 1e-9);
        // Seule la mensualité du crédit du foyer est comptée ; celle de la SCI ne l'est pas.
        assert!((f.mensualites - 1_050.0).abs() < 1e-9);
        assert!((f.e_mois - f.capacite_calculee).abs() < 1e-9);
        // (3 × 3200 + 2 × 2600) / 5800
        assert!((f.stab_ponderee - 14_800.0 / 5_800.0).abs() < 1e-9);
    }

    #[test]
    fn epargne_forcee_et_deficit() {
        let mut q = Questionnaire::exemple();
        q.epargne_mensuelle_forcee = Some(500.0);
        let h = Hypotheses::default();
        assert_eq!(q.foyer(&h).e_mois, 500.0);
        q.epargne_mensuelle_forcee = None;
        q.depenses.push(Depense { poste: PosteDepense::Autre, libelle: "x".into(), montant_mensuel: 10_000.0 });
        let f = q.foyer(&h);
        assert!(f.capacite_calculee < 0.0);
        assert_eq!(f.e_mois, 0.0);
    }
}
