//! Fiscalité et trésorerie des SCI.
//!
//! Deux régimes, deux logiques très différentes :
//!
//! - **SCI à l'IR** : la société est translucide. Le résultat foncier (loyers − charges −
//!   intérêts d'emprunt) est imposé chaque année chez les associés, à leur TMI plus les
//!   prélèvements sociaux. Le capital remboursé n'est pas déductible : c'est la cause
//!   classique d'un cash-flow négatif malgré un résultat imposable positif.
//! - **SCI à l'IS** : la société amortit le bien, ce qui écrase le résultat imposable les
//!   premières années. L'impôt est payé par la société. Les associés ne sont imposés
//!   (PFU) que sur ce qui est distribué : **le résultat conservé dans la SCI n'est pas un
//!   revenu passif du foyer**, même s'il fait grossir son patrimoine.
//!
//! Limites assumées : pas de plus-value de cession (à l'IS, les amortissements déduits
//! augmentent la plus-value taxable — une alerte le rappelle), pas de report de déficit
//! d'un exercice sur l'autre, régime réel supposé (pas de micro-foncier).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::credit::interets_sur;
use crate::model::{Dette, RegimeSci, Sci, Tmi};
use crate::params::Hypotheses;

/// Situation annuelle d'une SCI, du point de vue du foyer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BilanSci {
    pub id: String,
    pub nom: String,
    pub regime: RegimeSci,
    pub regime_libelle: String,
    pub part_foyer: f64,
    /// Valeur des biens + SCPI − capital restant dû, en quote-part du foyer.
    pub valeur_nette_eur: f64,
    /// Parts de SCPI détenues, en quote-part du foyer.
    pub scpi_eur: f64,
    pub loyers_annuels_eur: f64,
    pub charges_annuelles_eur: f64,
    pub interets_annuels_eur: f64,
    /// IS : dotation aux amortissements de l'exercice.
    pub amortissement_annuel_eur: f64,
    /// Assiette imposable : résultat foncier (IR) ou résultat fiscal (IS).
    pub resultat_imposable_eur: f64,
    pub impot_annuel_eur: f64,
    /// Trésorerie de la société après charges, crédit et impôt de la société.
    pub tresorerie_annuelle_eur: f64,
    /// Ce que le foyer touche réellement, net d'impôt, par mois.
    pub cash_flow_foyer_mensuel_eur: f64,
    /// IS : résultat conservé dans la société (patrimoine, mais pas un revenu passif).
    pub capitalise_annuel_eur: f64,
    /// Effort d'épargne mensuel que le foyer doit apporter (trésorerie négative).
    pub effort_mensuel_eur: f64,
    /// Mensualités de crédit portées par la SCI, en quote-part du foyer.
    pub mensualites_mensuelles_eur: f64,
    pub commentaires: Vec<String>,
}

/// Impôt sur les sociétés : taux réduit jusqu'au seuil, taux normal au-delà.
pub fn impot_societes(resultat: f64, h: &Hypotheses) -> f64 {
    if resultat <= 0.0 {
        return 0.0;
    }
    let reduit = resultat.min(h.is_seuil_taux_reduit);
    reduit * h.is_taux_reduit + (resultat - reduit) * h.is_taux_normal
}

fn eur(v: f64) -> String {
    crate::eur_public(v)
}

/// Bilan annuel d'une SCI, crédits rattachés compris.
pub fn bilan(sci: &Sci, dettes: &[Dette], tmi: Tmi, h: &Hypotheses) -> BilanSci {
    let part = sci.part_foyer();
    let credits: Vec<&Dette> = dettes.iter().filter(|d| d.sci_id.as_deref() == Some(&sci.id)).collect();

    let restant_du: f64 = credits.iter().map(|d| d.restant_du).fold(0.0, |a, b| a + b);
    let mensualites: f64 = credits.iter().map(|d| d.mensualite).fold(0.0, |a, b| a + b);
    let interets: f64 =
        credits.iter().map(|d| interets_sur(d.restant_du, d.taux(), d.mensualite, 12)).fold(0.0, |a, b| a + b);

    let loyers = sci.loyers_mensuels * 12.0;
    let charges = sci.charges_mensuelles * 12.0;
    let annuites = mensualites * 12.0;
    let mut commentaires = Vec::new();

    // Amortissement : seule une SCI à l'IS amortit, et seulement la part bâtie du bien
    // (le terrain ne s'amortit pas). Les SCPI ne sont pas amorties ici.
    let amortissement = match sci.regime {
        RegimeSci::Is if h.duree_amortissement_ans > 0 => {
            sci.valeur_biens * h.part_amortissable / h.duree_amortissement_ans as f64
        }
        _ => 0.0,
    };

    let (resultat, impot_societe, impot_foyer) = match sci.regime {
        RegimeSci::Ir => {
            let resultat = loyers - charges - interets;
            let impot = if resultat > 0.0 {
                resultat * (tmi.pct() as f64 / 100.0 + h.prelevements_sociaux)
            } else {
                let imputable = (-resultat).min(h.deficit_foncier_max);
                commentaires.push(format!(
                    "Déficit foncier de {} : imputable sur le revenu global à hauteur de {} par an, le reste sur les revenus fonciers des 10 années suivantes.",
                    eur(-resultat),
                    eur(imputable)
                ));
                0.0
            };
            // À l'IR l'impôt est dû par les associés, pas par la société.
            (resultat, 0.0, impot * part)
        }
        RegimeSci::Is => {
            let resultat = loyers - charges - interets - amortissement;
            let is = impot_societes(resultat, h);
            if resultat <= 0.0 && loyers > 0.0 {
                commentaires.push(
                    "Résultat fiscal négatif grâce à l'amortissement : aucun IS cette année, le déficit est reportable sans limite de durée."
                        .into(),
                );
            }
            (resultat, is, 0.0)
        }
    };

    // Trésorerie de la société : le capital remboursé sort de la trésorerie sans être
    // une charge déductible, l'amortissement est une charge sans sortie de trésorerie.
    let tresorerie = loyers - charges - annuites - impot_societe;

    let (cash_flow_foyer, capitalise) = match sci.regime {
        RegimeSci::Ir => ((tresorerie * part - impot_foyer).max(f64::MIN), 0.0),
        RegimeSci::Is => {
            let distribuable = tresorerie.max(0.0);
            let distribue = distribuable * (sci.distribution_pct / 100.0).clamp(0.0, 1.0);
            let net = distribue * part * (1.0 - h.pfu);
            if distribue < distribuable - 0.5 {
                commentaires.push(format!(
                    "{} restent dans la SCI : ils font grossir le patrimoine mais ne comptent pas comme revenu passif tant qu'ils ne sont pas distribués (PFU de {} à la distribution).",
                    eur((distribuable - distribue) * part),
                    crate::pct_public(h.pfu)
                ));
            }
            (net + (tresorerie.min(0.0)) * part, (distribuable - distribue) * part)
        }
    };

    if sci.regime == RegimeSci::Is && sci.valeur_biens > 0.0 {
        commentaires.push(
            "À l'IS, les amortissements déduits viennent augmenter la plus-value imposable en cas de revente : régime adapté à une détention longue."
                .into(),
        );
    }

    BilanSci {
        id: sci.id.clone(),
        nom: sci.nom.clone(),
        regime: sci.regime,
        regime_libelle: sci.regime.libelle().into(),
        part_foyer: part,
        valeur_nette_eur: ((sci.valeur_biens + sci.scpi) - restant_du) * part,
        scpi_eur: sci.scpi * part,
        loyers_annuels_eur: loyers,
        charges_annuelles_eur: charges,
        interets_annuels_eur: interets,
        amortissement_annuel_eur: amortissement,
        resultat_imposable_eur: resultat,
        impot_annuel_eur: impot_societe + impot_foyer / part.max(1e-9) * part,
        tresorerie_annuelle_eur: tresorerie,
        cash_flow_foyer_mensuel_eur: cash_flow_foyer / 12.0,
        capitalise_annuel_eur: capitalise,
        effort_mensuel_eur: (-cash_flow_foyer / 12.0).max(0.0),
        mensualites_mensuelles_eur: mensualites * part,
        commentaires,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ObjetCredit;

    fn sci(regime: RegimeSci) -> Sci {
        Sci {
            id: "sci-1".into(),
            nom: "SCI Test".into(),
            regime,
            part_foyer_pct: 100.0,
            valeur_biens: 300_000.0,
            scpi: 0.0,
            loyers_mensuels: 1_500.0,
            charges_mensuelles: 300.0,
            distribution_pct: 0.0,
        }
    }

    fn credit() -> Dette {
        Dette {
            libelle: "Crédit SCI".into(),
            taux_pct: 3.0,
            restant_du: 200_000.0,
            mensualite: 1_110.0,
            duree_restante_mois: 240,
            objet: ObjetCredit::Locatif,
            sci_id: Some("sci-1".into()),
        }
    }

    #[test]
    fn l_amortissement_efface_le_resultat_a_l_is() {
        let h = Hypotheses::default();
        let is = bilan(&sci(RegimeSci::Is), &[credit()], Tmi::T30, &h);
        let ir = bilan(&sci(RegimeSci::Ir), &[credit()], Tmi::T30, &h);
        // 300 000 × 85 % / 30 = 8 500 € par an, déductibles seulement à l'IS.
        assert!((is.amortissement_annuel_eur - 8_500.0).abs() < 1.0);
        assert_eq!(ir.amortissement_annuel_eur, 0.0);
        // Même bien, même crédit : l'IS ramène le résultat imposable à presque rien.
        assert!((ir.resultat_imposable_eur - is.resultat_imposable_eur - 8_500.0).abs() < 1.0);
        assert!(is.resultat_imposable_eur.abs() < 100.0, "{}", is.resultat_imposable_eur);
        assert!(is.impot_annuel_eur < 20.0);
        assert!(ir.impot_annuel_eur > 3_000.0);
    }

    #[test]
    fn a_l_ir_l_impot_peut_depasser_la_tresorerie() {
        let h = Hypotheses::default();
        let b = bilan(&sci(RegimeSci::Ir), &[credit()], Tmi::T30, &h);
        // Le capital remboursé n'est pas déductible : le résultat imposable reste élevé…
        assert!(b.resultat_imposable_eur > 8_000.0);
        assert!(b.tresorerie_annuelle_eur > 0.0);
        // … au point que l'impôt dépasse la trésorerie dégagée : le foyer doit remettre au pot.
        assert!(b.impot_annuel_eur > b.tresorerie_annuelle_eur);
        assert!(b.cash_flow_foyer_mensuel_eur < 0.0);
        assert!(b.effort_mensuel_eur > 0.0);
    }

    #[test]
    fn is_non_distribue_n_est_pas_un_revenu_passif() {
        let h = Hypotheses::default();
        let mut s = sci(RegimeSci::Is);
        s.loyers_mensuels = 3_000.0;
        let b = bilan(&s, &[credit()], Tmi::T30, &h);
        assert!(b.tresorerie_annuelle_eur > 0.0);
        // Rien n'est distribué : le foyer ne touche rien, tout est capitalisé.
        assert_eq!(b.cash_flow_foyer_mensuel_eur, 0.0);
        assert!(b.capitalise_annuel_eur > 0.0);

        let mut s2 = s.clone();
        s2.distribution_pct = 100.0;
        let b2 = bilan(&s2, &[credit()], Tmi::T30, &h);
        assert!(b2.cash_flow_foyer_mensuel_eur > 0.0);
        // Distribution nette de PFU.
        assert!((b2.cash_flow_foyer_mensuel_eur * 12.0 - b2.tresorerie_annuelle_eur * (1.0 - h.pfu)).abs() < 1.0);
        assert_eq!(b2.capitalise_annuel_eur, 0.0);
    }

    #[test]
    fn quote_part_appliquee() {
        let h = Hypotheses::default();
        let mut s = sci(RegimeSci::Ir);
        s.part_foyer_pct = 50.0;
        let b = bilan(&s, &[credit()], Tmi::T30, &h);
        assert!((b.valeur_nette_eur - (300_000.0 - 200_000.0) * 0.5).abs() < 1e-6);
        assert!((b.mensualites_mensuelles_eur - 555.0).abs() < 1e-6);
    }

    #[test]
    fn bareme_is_a_deux_taux() {
        let h = Hypotheses::default();
        assert_eq!(impot_societes(-1_000.0, &h), 0.0);
        assert!((impot_societes(10_000.0, &h) - 1_500.0).abs() < 1e-6);
        // 42 500 × 15 % + 7 500 × 25 %
        assert!((impot_societes(50_000.0, &h) - (6_375.0 + 1_875.0)).abs() < 1e-6);
    }
}
