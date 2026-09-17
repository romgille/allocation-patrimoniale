//! Poche 0 (précaution) et poche 1 (sous-poches datées : études des enfants
//! et projets du foyer).

use crate::finance::pmt;
use crate::model::{Enfant, Foyer, Priorite, Projet, Questionnaire, StatutRp, TypeEcole};
use crate::output::{AllocationEtudes, CategoriePoche, Poche0, SousPoche};
use crate::params::Hypotheses;

pub fn poche0(q: &Questionnaire, f: &Foyer, h: &Hypotheses) -> Poche0 {
    let cible_mois = if f.stab_ponderee <= 1.5 {
        h.precaution_mois_max
    } else if q.rp == StatutRp::ProprietairePayee && f.stab_ponderee >= 2.5 {
        h.precaution_mois_min
    } else {
        h.precaution_mois_defaut
    };
    // Les dépenses de référence incluent les mensualités de crédit.
    let cible_eur = cible_mois * (f.depenses + f.mensualites);
    let existant = q.avoirs.livrets;
    let mut supports = Vec::new();
    if q.lep_eligible {
        supports.push("LEP (prioritaire)".to_string());
    }
    supports.extend(["Livret A".to_string(), "LDDS".to_string()]);
    supports.push("Fonds monétaire (si besoin)".to_string());
    Poche0 {
        cible_mois,
        cible_eur,
        existant_eur: existant,
        manque_eur: (cible_eur - existant).max(0.0),
        excedent_eur: (existant - cible_eur).max(0.0),
        supports,
    }
}

pub fn frais_scolarite(t: TypeEcole, h: &Hypotheses) -> f64 {
    match t {
        TypeEcole::Public => h.frais_scolarite_public,
        TypeEcole::Prive => h.frais_scolarite_prive,
        TypeEcole::Mixte => h.frais_scolarite_mixte,
    }
}

struct Base {
    nom: String,
    categorie: CategoriePoche,
    priorite: Priorite,
    horizon: u32,
    montant_actuel: f64,
    cout_annuel: Option<f64>,
    capital_cible: f64,
    capital_actuel: f64,
}

fn construire(b: Base, h: &Hypotheses) -> SousPoche {
    let part_actions = h.part_actions_glide(b.horizon);
    let r = h.rendement_actions * part_actions + h.rendement_securise * (1.0 - part_actions);

    let (epargne, deficit) = if b.horizon == 0 {
        (0.0, (b.capital_cible - b.capital_actuel).max(0.0))
    } else {
        (pmt(b.capital_cible, b.capital_actuel, r, b.horizon), 0.0)
    };

    // Prochain palier : l'horizon passe sous le seuil du palier courant.
    let (prochain_dans, prochaine_part) = match h.palier_glide(b.horizon) {
        Some(p) if p.horizon_min_ans > 0 || p.part_actions > 0.0 => {
            let dans = b.horizon - p.horizon_min_ans + 1;
            let next_h = b.horizon.saturating_sub(dans);
            (Some(dans), Some(h.part_actions_glide(next_h)))
        }
        _ => (None, None),
    };

    let enveloppe = if b.horizon == 0 {
        "Fonds euros / livret (capital sécurisé, disponible)".to_string()
    } else if b.categorie == CategoriePoche::Etudes {
        "Assurance-vie à frais bas (fonds euros + ETF monde), ouverte tôt pour prendre date ; \
         option AV au nom de l'enfant alimentée par dons"
            .to_string()
    } else if b.horizon < 5 {
        "Livret / fonds euros d'assurance-vie (horizon court)".to_string()
    } else {
        "Assurance-vie à frais bas (fonds euros + ETF monde)".to_string()
    };

    SousPoche {
        nom: b.nom,
        categorie: b.categorie,
        priorite: b.priorite,
        horizon_ans: b.horizon,
        montant_actuel_eur: b.montant_actuel,
        cout_annuel_actuel_eur: b.cout_annuel,
        capital_cible_eur: b.capital_cible,
        capital_actuel_eur: b.capital_actuel,
        epargne_mensuelle_eur: epargne,
        deficit_immediat_eur: deficit,
        rendement_attendu: r,
        allocation: AllocationEtudes { etf_monde: part_actions, fonds_euros: 1.0 - part_actions },
        prochain_palier_dans_ans: prochain_dans,
        prochaine_part_actions: prochaine_part,
        enveloppe,
    }
}

pub fn sous_poche_etudes(e: &Enfant, h: &Hypotheses) -> SousPoche {
    let horizon = h.age_debut_etudes.saturating_sub(e.age);
    let cout_annuel = frais_scolarite(e.type_ecole, h)
        + if e.logement_etudiant { h.cout_logement_annuel } else { 0.0 };
    let montant = cout_annuel * e.duree_etudes as f64;
    construire(
        Base {
            nom: format!("Études — {}", e.prenom),
            categorie: CategoriePoche::Etudes,
            priorite: Priorite::Essentiel,
            horizon,
            montant_actuel: montant,
            cout_annuel: Some(cout_annuel),
            capital_cible: montant * (1.0 + h.inflation_etudes).powi(horizon as i32),
            capital_actuel: e.capital_deja_affecte,
        },
        h,
    )
}

pub fn sous_poche_projet(p: &Projet, h: &Hypotheses) -> SousPoche {
    construire(
        Base {
            nom: p.libelle.clone(),
            categorie: CategoriePoche::Projet,
            priorite: p.priorite,
            horizon: p.dans_ans,
            montant_actuel: p.montant,
            cout_annuel: None,
            capital_cible: p.montant * (1.0 + h.inflation_projets).powi(p.dans_ans as i32),
            capital_actuel: p.capital_deja_affecte,
        },
        h,
    )
}

/// Toutes les sous-poches, triées par priorité puis par échéance.
pub fn sous_poches(q: &Questionnaire, h: &Hypotheses) -> Vec<SousPoche> {
    let mut v: Vec<SousPoche> = q
        .enfants
        .iter()
        .map(|e| sous_poche_etudes(e, h))
        .chain(q.projets.iter().map(|p| sous_poche_projet(p, h)))
        .collect();
    v.sort_by_key(|s| (s.priorite, s.horizon_ans));
    v
}

/// Répartit `montant` entre les sous-poches par niveau de priorité :
/// chaque niveau est servi entièrement avant le suivant, au prorata à l'intérieur d'un niveau.
pub fn repartir_par_priorite(montant: f64, poches: &[SousPoche]) -> Vec<f64> {
    let mut reste = montant.max(0.0);
    let mut out = vec![0.0; poches.len()];
    for niveau in [Priorite::Essentiel, Priorite::Important, Priorite::Souhaitable] {
        let idx: Vec<usize> = (0..poches.len()).filter(|i| poches[*i].priorite == niveau).collect();
        let besoin: f64 = idx.iter().map(|i| poches[*i].epargne_mensuelle_eur).fold(0.0, |a, b| a + b);
        if besoin <= 0.0 {
            continue;
        }
        let ratio = (reste / besoin).min(1.0);
        for i in idx {
            out[i] = poches[i].epargne_mensuelle_eur * ratio;
        }
        reste -= besoin * ratio;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::finance::fv;

    fn enfant(age: u32) -> Enfant {
        Enfant {
            prenom: "A".into(),
            age,
            type_ecole: TypeEcole::Public,
            logement_etudiant: true,
            duree_etudes: 5,
            capital_deja_affecte: 0.0,
        }
    }

    #[test]
    fn glide_path_bornes() {
        let h = Hypotheses::default();
        assert_eq!(h.part_actions_glide(13), 0.8);
        assert_eq!(h.part_actions_glide(12), 0.6);
        assert_eq!(h.part_actions_glide(8), 0.6);
        assert_eq!(h.part_actions_glide(7), 0.4);
        assert_eq!(h.part_actions_glide(5), 0.4);
        assert_eq!(h.part_actions_glide(4), 0.2);
        assert_eq!(h.part_actions_glide(3), 0.2);
        assert_eq!(h.part_actions_glide(2), 0.0);
        assert_eq!(h.part_actions_glide(0), 0.0);
    }

    #[test]
    fn cout_et_epargne() {
        let h = Hypotheses::default();
        let s = sous_poche_etudes(&enfant(8), &h);
        assert_eq!(s.horizon_ans, 10);
        assert_eq!(s.priorite, Priorite::Essentiel);
        // (3000 + 9000) × 5 × 1,03^10
        let attendu = 60_000.0 * 1.03f64.powi(10);
        assert!((s.capital_cible_eur - attendu).abs() < 1e-6);
        assert!((s.rendement_attendu - (0.6 * 0.065 + 0.4 * 0.025)).abs() < 1e-12);
        let v = fv(0.0, s.epargne_mensuelle_eur, s.rendement_attendu, 120);
        assert!((v - attendu).abs() < 0.01);
        assert_eq!(s.prochain_palier_dans_ans, Some(3));
        assert_eq!(s.prochaine_part_actions, Some(0.4));
    }

    #[test]
    fn enfant_majeur() {
        let h = Hypotheses::default();
        let s = sous_poche_etudes(&enfant(19), &h);
        assert_eq!(s.horizon_ans, 0);
        assert_eq!(s.epargne_mensuelle_eur, 0.0);
        assert!(s.deficit_immediat_eur > 0.0);
        assert_eq!(s.allocation.etf_monde, 0.0);
        assert_eq!(s.prochain_palier_dans_ans, None);
    }

    #[test]
    fn projet_date() {
        let h = Hypotheses::default();
        let p = Projet {
            libelle: "Voiture".into(),
            montant: 10_000.0,
            dans_ans: 3,
            priorite: Priorite::Important,
            capital_deja_affecte: 0.0,
        };
        let s = sous_poche_projet(&p, &h);
        assert!((s.capital_cible_eur - 10_000.0 * 1.02f64.powi(3)).abs() < 1e-6);
        assert_eq!(s.allocation.etf_monde, 0.2);
        assert!(s.enveloppe.contains("horizon court"));
        assert_eq!(s.cout_annuel_actuel_eur, None);
    }

    #[test]
    fn priorites_servies_dans_l_ordre() {
        let h = Hypotheses::default();
        let mk = |prio, m| {
            let mut s = sous_poche_projet(
                &Projet { libelle: "p".into(), montant: 1.0, dans_ans: 5, priorite: prio, capital_deja_affecte: 0.0 },
                &h,
            );
            s.epargne_mensuelle_eur = m;
            s
        };
        let poches = vec![mk(Priorite::Essentiel, 100.0), mk(Priorite::Important, 100.0), mk(Priorite::Important, 300.0), mk(Priorite::Souhaitable, 50.0)];
        let r = repartir_par_priorite(300.0, &poches);
        assert_eq!(r[0], 100.0);
        assert!((r[1] - 50.0).abs() < 1e-9 && (r[2] - 150.0).abs() < 1e-9);
        assert_eq!(r[3], 0.0);
        let r = repartir_par_priorite(10_000.0, &poches);
        assert_eq!(r, vec![100.0, 100.0, 300.0, 50.0]);
    }
}
