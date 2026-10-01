//! Vente d'un bien immobilier : plus-value, impôt et produit net encaissé.
//!
//! Deux fiscalités très différentes selon qui détient le bien :
//!
//! - **Particulier ou SCI à l'IR** — plus-value des particuliers. La base est le prix de
//!   cession moins le prix d'acquisition, réduite par des abattements pour durée de
//!   détention, différents pour l'impôt (exonération à 22 ans) et pour les prélèvements
//!   sociaux (exonération à 30 ans). Une surtaxe s'ajoute au-delà de 50 000 € de plus-value.
//! - **SCI à l'IS** — plus-value professionnelle. La base est le prix de cession moins la
//!   *valeur nette comptable*, c'est-à-dire le prix d'acquisition diminué des amortissements
//!   déjà déduits. Tout ce qui a été amorti pendant la détention est donc réintégré et
//!   imposé au taux de l'IS : l'avantage pris chaque année se paie à la sortie.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::credit::interets_sur;
use crate::model::{BienVendu, Dette, Questionnaire, RegimeSci, VenteImmobiliere};
use crate::params::Hypotheses;
use crate::sci::impot_societes;

/// Produit d'une vente programmée, du point de vue du foyer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Cession {
    pub libelle: String,
    pub bien_libelle: String,
    pub dans_ans: u32,
    pub part_vendue: f64,
    /// Prix de vente retenu à l'échéance (prix d'aujourd'hui revalorisé).
    pub prix_vente_eur: f64,
    pub frais_vente_eur: f64,
    /// Capital restant dû du ou des crédits soldés à la vente.
    pub credit_solde_eur: f64,
    /// Base imposable : prix − prix d'acquisition (IR) ou − valeur nette comptable (IS).
    pub plus_value_brute_eur: f64,
    /// IS : amortissements réintégrés dans la plus-value.
    pub amortissements_reintegres_eur: f64,
    /// Abattement pour durée de détention appliqué (0 à l'IS).
    pub abattement_eur: f64,
    pub impot_eur: f64,
    pub regime_libelle: String,
    /// Ce qui reste au foyer, après frais, crédit et impôt.
    pub produit_net_eur: f64,
    /// Revenu passif mensuel qui disparaît avec le bien.
    pub cash_flow_perdu_mensuel_eur: f64,
    /// Mensualités qui cessent d'être dues (crédit soldé).
    pub mensualites_liberees_eur: f64,
    pub commentaires: Vec<String>,
}

/// Abattement pour durée de détention (plus-value des particuliers).
/// Renvoie la part de la plus-value qui reste imposable, pour l'impôt puis pour les
/// prélèvements sociaux.
pub fn abattements_particulier(annees: u32) -> (f64, f64) {
    // Impôt sur le revenu : 6 %/an de la 6e à la 21e année, 4 % la 22e, exonéré ensuite.
    let ir = if annees >= 22 {
        0.0
    } else if annees >= 6 {
        (1.0 - (annees - 5) as f64 * 0.06 - if annees >= 22 { 0.04 } else { 0.0 }).max(0.0)
    } else {
        1.0
    };
    // Prélèvements sociaux : 1,65 %/an de la 6e à la 21e, 1,60 % la 22e, 9 %/an ensuite.
    let ps = if annees >= 30 {
        0.0
    } else if annees >= 23 {
        (1.0 - 16.0 * 0.0165 - 0.016 - (annees - 22) as f64 * 0.09).max(0.0)
    } else if annees == 22 {
        1.0 - 16.0 * 0.0165 - 0.016
    } else if annees >= 6 {
        (1.0 - (annees - 5) as f64 * 0.0165).max(0.0)
    } else {
        1.0
    };
    (ir.clamp(0.0, 1.0), ps.clamp(0.0, 1.0))
}

/// Surtaxe sur les plus-values immobilières supérieures à 50 000 € (barème simplifié :
/// progression par tranches de 2 % à 6 %).
fn surtaxe(plus_value_imposable: f64) -> f64 {
    let pv = plus_value_imposable;
    let taux = if pv <= 50_000.0 {
        0.0
    } else if pv <= 100_000.0 {
        0.02
    } else if pv <= 150_000.0 {
        0.03
    } else if pv <= 200_000.0 {
        0.04
    } else if pv <= 250_000.0 {
        0.05
    } else {
        0.06
    };
    pv * taux
}

fn eur(v: f64) -> String {
    crate::eur_public(v)
}

/// Capital restant dû d'un crédit après `mois` mensualités.
fn capital_restant(d: &Dette, mois: u32) -> f64 {
    let i = d.taux() / 12.0;
    let mut capital = d.restant_du;
    for m in 0..mois {
        if capital <= 0.0 {
            return 0.0;
        }
        let mensualite = d.mensualite_a(m);
        capital = (capital + capital * i - mensualite).max(0.0);
    }
    capital
}

/// Calcule le produit d'une vente programmée.
pub fn cession(q: &Questionnaire, v: &VenteImmobiliere, h: &Hypotheses) -> Cession {
    let part = v.part_vendue();
    let mois = v.dans_ans * 12;
    let annees = v.detention_a_la_vente();
    let revalorisation = (1.0 + h.revalorisation_immobilier).powi(v.dans_ans as i32);
    let prix = v.prix_vente * revalorisation * part;
    let frais = prix * (v.frais_vente_pct / 100.0);
    let acquisition = v.prix_acquisition * part;

    // Crédits adossés au bien, et leur capital restant dû à la date de la vente.
    let credits: Vec<&Dette> = match &v.bien {
        BienVendu::LocatifDirect => {
            q.dettes_foyer().filter(|d| d.objet == crate::model::ObjetCredit::Locatif).collect()
        }
        BienVendu::Sci { id } => q.dettes.iter().filter(|d| d.sci_id.as_deref() == Some(id.as_str())).collect(),
    };
    let credit_solde: f64 = credits.iter().map(|d| capital_restant(d, mois)).fold(0.0, |a, b| a + b) * part;
    let mensualites_liberees: f64 =
        credits.iter().map(|d| d.mensualite_a(mois)).fold(0.0, |a, b| a + b) * part;

    let sci = match &v.bien {
        BienVendu::Sci { id } => q.scis.iter().find(|s| &s.id == id),
        BienVendu::LocatifDirect => None,
    };
    let quote_part = sci.map(|s| s.part_foyer()).unwrap_or(1.0);
    let mut commentaires = Vec::new();

    let is = sci.map(|s| s.regime) == Some(RegimeSci::Is);
    let (base, amortissements, abattement, impot, regime) = if is {
        // Valeur nette comptable : le prix d'acquisition moins tout ce qui a été amorti.
        let amort_annuel = if h.duree_amortissement_ans > 0 {
            sci.expect("SCI présente").base_amortissement * part * h.part_amortissable / h.duree_amortissement_ans as f64
        } else {
            0.0
        };
        let cumul = (amort_annuel * annees as f64)
            .min(sci.expect("SCI présente").base_amortissement * part * h.part_amortissable);
        let vnc = (acquisition - cumul).max(0.0);
        let pv = (prix - frais - vnc).max(0.0);
        let impot = impot_societes(pv, h);
        commentaires.push(format!(
            "SCI à l'IS : {} d'amortissements déduits depuis l'acquisition sont réintégrés dans la plus-value. Sans eux, l'impôt de cession serait de {}.",
            eur(cumul),
            eur(impot_societes((prix - frais - acquisition).max(0.0), h))
        ));
        commentaires.push(
            "Aucun abattement pour durée de détention à l'IS : le régime reste avantageux tant que le bien n'est pas vendu.".into(),
        );
        (pv, cumul, 0.0, impot, "Plus-value professionnelle (IS)")
    } else {
        let brute = (prix - frais - acquisition).max(0.0);
        let (part_ir, part_ps) = abattements_particulier(annees);
        let assiette_ir = brute * part_ir;
        let assiette_ps = brute * part_ps;
        let impot = assiette_ir * h.taux_pv_immobiliere
            + assiette_ps * h.prelevements_sociaux
            + surtaxe(assiette_ir);
        if part_ir == 0.0 && part_ps == 0.0 {
            commentaires.push(format!("Détention de {annees} ans : plus-value totalement exonérée."));
        } else if part_ir < 1.0 {
            commentaires.push(format!(
                "Détention de {} ans : abattement de {} sur l'impôt et de {} sur les prélèvements sociaux. Exonération totale à 22 ans (impôt) et 30 ans (prélèvements sociaux).",
                annees,
                crate::pct_public(1.0 - part_ir),
                crate::pct_public(1.0 - part_ps)
            ));
        }
        (brute, 0.0, brute - assiette_ir, impot, "Plus-value des particuliers")
    };

    // À l'IR comme en direct, l'impôt est dû par les associés au prorata ; à l'IS il est
    // payé par la société avant que le solde ne remonte au foyer.
    let produit_societe = prix - frais - credit_solde - if is { impot } else { 0.0 };
    let produit_net = if is {
        let distribution = sci.expect("SCI présente").distribution_pct / 100.0;
        if produit_societe < 0.0 {
            produit_societe * quote_part
        } else {
            produit_societe * distribution * quote_part * (1.0 - h.pfu)
        }
    } else {
        (produit_societe - impot) * quote_part
    };

    let cash_flow_perdu = match &v.bien {
        BienVendu::LocatifDirect => q.cf_immo.max(0.0) * part,
        BienVendu::Sci { id } => q
            .bilans_scis(h)
            .iter()
            .find(|b| &b.id == id)
            .map(|b| b.cash_flow_foyer_mensuel_eur.max(0.0))
            .unwrap_or(0.0)
            * part,
    };

    if credit_solde > prix - frais {
        commentaires.push(
            "Le capital restant dû dépasse le prix de vente net : la vente ne dégage rien et laisse une dette à combler.".into(),
        );
    }

    Cession {
        libelle: v.libelle.clone(),
        bien_libelle: match &v.bien {
            BienVendu::LocatifDirect => "Locatif en direct".into(),
            BienVendu::Sci { id } => {
                q.scis.iter().find(|s| &s.id == id).map(|s| s.nom.clone()).unwrap_or_else(|| "SCI".into())
            }
        },
        dans_ans: v.dans_ans,
        part_vendue: part,
        prix_vente_eur: prix * quote_part,
        frais_vente_eur: frais * quote_part,
        credit_solde_eur: credit_solde * quote_part,
        plus_value_brute_eur: base * quote_part,
        amortissements_reintegres_eur: amortissements * quote_part,
        abattement_eur: abattement * quote_part,
        impot_eur: impot * quote_part,
        regime_libelle: regime.into(),
        produit_net_eur: produit_net,
        cash_flow_perdu_mensuel_eur: cash_flow_perdu,
        mensualites_liberees_eur: mensualites_liberees * quote_part,
        commentaires,
    }
}

/// Intérêts d'emprunt encore dus sur 12 mois pour un crédit donné : utilisé par les tests
/// de cohérence entre la vente et le bilan de la SCI.
pub fn interets_annuels(d: &Dette) -> f64 {
    interets_sur(d.restant_du, d.taux(), d.mensualite, 12)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{BienVendu, ObjetCredit, RegimeSci, Sci};

    #[test]
    fn abattements_aux_bornes() {
        assert_eq!(abattements_particulier(5), (1.0, 1.0));
        assert_eq!(abattements_particulier(0), (1.0, 1.0));
        // 22 ans : exonération d'impôt, prélèvements sociaux encore dus en partie.
        let (ir, ps) = abattements_particulier(22);
        assert_eq!(ir, 0.0);
        // 16 × 1,65 % + 1,60 % = 28 % d'abattement : 72 % encore soumis aux prélèvements.
        assert!((ps - 0.72).abs() < 1e-9, "{ps}");
        // 30 ans : exonération totale.
        assert_eq!(abattements_particulier(30), (0.0, 0.0));
        // Décroissance monotone.
        for a in 6..30 {
            let (i1, p1) = abattements_particulier(a);
            let (i2, p2) = abattements_particulier(a + 1);
            assert!(i2 <= i1 + 1e-9 && p2 <= p1 + 1e-9, "année {a}");
        }
    }

    fn q_avec_vente(v: VenteImmobiliere) -> Questionnaire {
        let mut q = Questionnaire::exemple();
        q.ventes = vec![v];
        q
    }

    #[test]
    fn vente_en_direct_apres_longue_detention() {
        let h = Hypotheses::default();
        let q = q_avec_vente(VenteImmobiliere {
            libelle: "Vente".into(),
            bien: BienVendu::LocatifDirect,
            dans_ans: 0,
            part_vendue_pct: 100.0,
            prix_vente: 200_000.0,
            prix_acquisition: 120_000.0,
            detention_ans: 30,
            frais_vente_pct: 0.0,
        });
        let c = cession(&q, &q.ventes[0], &h);
        assert_eq!(c.impot_eur, 0.0);
        assert!((c.produit_net_eur - 200_000.0).abs() < 1.0);
    }

    #[test]
    fn vente_recente_imposee() {
        let h = Hypotheses::default();
        let q = q_avec_vente(VenteImmobiliere {
            libelle: "Vente".into(),
            bien: BienVendu::LocatifDirect,
            dans_ans: 0,
            part_vendue_pct: 100.0,
            prix_vente: 200_000.0,
            prix_acquisition: 120_000.0,
            detention_ans: 2,
            frais_vente_pct: 0.0,
        });
        let c = cession(&q, &q.ventes[0], &h);
        // Aucun abattement avant 6 ans : 80 000 € imposés, plus la surtaxe de 2 % au-delà
        // de 50 000 €.
        let attendu = 80_000.0 * (h.taux_pv_immobiliere + h.prelevements_sociaux) + 1_600.0;
        assert!((c.impot_eur - attendu).abs() < 1.0, "{}", c.impot_eur);
        assert!(c.produit_net_eur < 172_000.0);
    }

    #[test]
    fn is_reintegre_les_amortissements() {
        let h = Hypotheses::default();
        let mut q = Questionnaire::exemple();
        q.scis = vec![Sci {
            id: "s".into(),
            nom: "SCI".into(),
            regime: RegimeSci::Is,
            part_foyer_pct: 100.0,
            valeur_biens: 300_000.0,
            base_amortissement: 300_000.0,
            scpi: 0.0,
            loyers_mensuels: 1_200.0,
            charges_mensuelles: 200.0,
            distribution_pct: 0.0,
        }];
        q.dettes = vec![];
        q.ventes = vec![VenteImmobiliere {
            libelle: "Vente SCI".into(),
            bien: BienVendu::Sci { id: "s".into() },
            dans_ans: 0,
            part_vendue_pct: 100.0,
            prix_vente: 300_000.0,
            prix_acquisition: 300_000.0,
            detention_ans: 10,
            frais_vente_pct: 0.0,
        }];
        let c = cession(&q, &q.ventes[0], &h);
        // Vendu au prix d'achat : aucune plus-value pour un particulier, mais à l'IS la
        // valeur nette comptable a baissé de 10 ans d'amortissements (85 000 €).
        assert!((c.amortissements_reintegres_eur - 85_000.0).abs() < 1.0);
        assert!((c.plus_value_brute_eur - 85_000.0).abs() < 1.0);
        assert!(c.impot_eur > 0.0);
    }

    #[test]
    fn credit_solde_a_la_vente() {
        let h = Hypotheses::default();
        let mut q = Questionnaire::exemple();
        q.dettes = vec![Dette {
            libelle: "Crédit locatif".into(),
            taux_pct: 2.0,
            restant_du: 100_000.0,
            mensualite: 600.0,
            duree_restante_mois: 200,
            objet: ObjetCredit::Locatif,
            sci_id: None,
        }];
        q.ventes = vec![VenteImmobiliere {
            libelle: "Vente".into(),
            bien: BienVendu::LocatifDirect,
            dans_ans: 5,
            part_vendue_pct: 100.0,
            prix_vente: 200_000.0,
            prix_acquisition: 200_000.0,
            detention_ans: 25,
            frais_vente_pct: 0.0,
        }];
        let c = cession(&q, &q.ventes[0], &h);
        // Le capital a été amorti pendant 5 ans : il reste moins que 100 000 €.
        assert!(c.credit_solde_eur > 0.0 && c.credit_solde_eur < 100_000.0);
        assert!((c.mensualites_liberees_eur - 600.0).abs() < 1e-6);
        assert!(c.produit_net_eur > 100_000.0);
    }
}
