//! Validation des entrées et des hypothèses.

use crate::model::Questionnaire;
use crate::params::{Hypotheses, Ligne};

fn montant(err: &mut Vec<String>, nom: &str, v: f64) {
    if !v.is_finite() || v < 0.0 {
        err.push(format!("{nom} doit être un montant positif ou nul."));
    }
}

fn fraction(err: &mut Vec<String>, nom: &str, v: f64) {
    if !v.is_finite() || !(0.0..=1.0).contains(&v) {
        err.push(format!("{nom} doit être compris entre 0 et 100 %."));
    }
}

pub fn valider(q: &Questionnaire, h: &Hypotheses) -> Result<(), Vec<String>> {
    let mut e = Vec::new();

    if q.adultes.is_empty() {
        e.push("Le foyer doit compter au moins un adulte.".into());
    }
    if q.adultes.len() > 6 {
        e.push("6 adultes maximum.".into());
    }
    for (i, a) in q.adultes.iter().enumerate() {
        let n = if a.prenom.trim().is_empty() { format!("Adulte {}", i + 1) } else { a.prenom.clone() };
        if !(16..=100).contains(&a.age) {
            e.push(format!("{n} : l'âge doit être compris entre 16 et 100 ans."));
        }
        if !(1..=5).contains(&a.tol_risque) {
            e.push(format!("{n} : la tolérance au risque doit être comprise entre 1 et 5."));
        }
        if !(1..=3).contains(&a.stab_revenus) {
            e.push(format!("{n} : la stabilité des revenus doit être comprise entre 1 et 3."));
        }
        for r in &a.revenus {
            montant(&mut e, &format!("{n} : le revenu « {} »", r.libelle), r.montant_mensuel);
        }
    }
    for d in &q.depenses {
        montant(&mut e, &format!("La dépense « {} »", d.libelle), d.montant_mensuel);
    }
    if let Some(x) = q.epargne_mensuelle_forcee {
        montant(&mut e, "L'épargne mensuelle imposée", x);
    }
    if q.projets.len() > 30 {
        e.push("30 projets maximum.".into());
    }
    for (i, p) in q.projets.iter().enumerate() {
        let n = if p.libelle.trim().is_empty() { format!("Projet {}", i + 1) } else { p.libelle.clone() };
        if p.dans_ans > 60 {
            e.push(format!("{n} : échéance ≤ 60 ans."));
        }
        montant(&mut e, &format!("{n} : le montant"), p.montant);
        montant(&mut e, &format!("{n} : le capital déjà affecté"), p.capital_deja_affecte);
    }
    if q.h_fire > 60 {
        e.push("L'horizon revenus passifs doit être ≤ 60 ans.".into());
    }
    if !(q.taux_retrait_pct > 0.0 && q.taux_retrait_pct <= 10.0) {
        e.push("Le taux de retrait doit être compris entre 0 et 10 %.".into());
    }
    if !(0.0..=100.0).contains(&q.contraintes.crypto_souhaitee_pct) {
        e.push("La part de crypto souhaitée doit être comprise entre 0 et 100 %.".into());
    }
    for (nom, v) in [
        ("Le revenu passif visé", q.r_cible),
        ("Les autres revenus passifs", q.autres_revenus_passifs),
        ("La valeur nette du locatif", q.v_immo_loc),
        ("Les livrets", q.avoirs.livrets),
        ("Les liquidités à investir", q.avoirs.liquidites_a_investir),
        ("Les ETF monde", q.avoirs.etf_monde),
        ("Les fonds euros / obligations", q.avoirs.fonds_euros_obligations),
        ("Les SCPI", q.avoirs.scpi),
        ("L'or", q.avoirs.or),
        ("Les actions en direct", q.avoirs.actions_directes),
        ("Le crowdfunding immobilier", q.avoirs.crowdfunding_immo),
        ("Le crowdfunding ENR", q.avoirs.crowdfunding_enr),
        ("La crypto", q.avoirs.crypto),
        ("Le private equity", q.avoirs.private_equity),
    ] {
        montant(&mut e, nom, v);
    }
    if !q.cf_immo.is_finite() {
        e.push("Le cash-flow immobilier doit être un nombre.".into());
    }
    if q.enfants.len() > 12 {
        e.push("12 enfants maximum.".into());
    }
    for (i, enf) in q.enfants.iter().enumerate() {
        let n = if enf.prenom.trim().is_empty() { format!("Enfant {}", i + 1) } else { enf.prenom.clone() };
        if enf.age > 30 {
            e.push(format!("{n} : âge ≤ 30 ans."));
        }
        if !(1..=10).contains(&enf.duree_etudes) {
            e.push(format!("{n} : durée d'études entre 1 et 10 ans."));
        }
        montant(&mut e, &format!("{n} : le capital déjà affecté"), enf.capital_deja_affecte);
    }
    for d in &q.dettes {
        if !(0.0..=30.0).contains(&d.taux_pct) {
            e.push(format!("Crédit « {} » : taux entre 0 et 30 %.", d.libelle));
        }
        montant(&mut e, &format!("Crédit « {} » : le restant dû", d.libelle), d.restant_du);
        montant(&mut e, &format!("Crédit « {} » : la mensualité", d.libelle), d.mensualite);
    }

    // --- Hypothèses ---
    for (nom, v) in [
        ("Inflation études", h.inflation_etudes),
        ("Inflation projets", h.inflation_projets),
        ("Rendement actions", h.rendement_actions),
        ("Rendement sécurisé", h.rendement_securise),
        ("Seuil taux de dette", h.seuil_taux_dette),
        ("Endettement max", h.endettement_max),
        ("Plafond actions (tolérance faible)", h.actions_max_tolerance_faible),
        ("Plancher fonds euros", h.plancher_fonds_euros),
        ("Plancher fonds euros horizon court", h.plancher_fonds_euros_horizon_court),
        ("Or min", h.or_min),
        ("Or max", h.or_max),
        ("Crowdfunding max", h.crowdfunding_max),
        ("Actions directes max", h.actions_directes_max),
        ("Satellites max", h.satellites_max),
        ("Crypto max", h.crypto_max),
        ("Immobilier max", h.immo_max),
        ("Immobilier max (gestion élevée)", h.immo_max_gestion_elevee),
        ("Seuil de concentration immobilière", h.seuil_concentration_immo),
        ("Tolérance de rééquilibrage", h.tolerance_reequilibrage),
        ("PFU", h.pfu),
        ("Prélèvements sociaux", h.prelevements_sociaux),
        ("Plafond PER", h.plafond_per_revenus),
    ] {
        fraction(&mut e, nom, v);
    }
    if h.or_min > h.or_max {
        e.push("Or min doit être ≤ or max.".into());
    }
    if h.plancher_fonds_euros + h.or_min > 1.0 {
        e.push("Les planchers (fonds euros + or) dépassent 100 %.".into());
    }
    if h.seuil_score_prudent > h.seuil_score_dynamique {
        e.push("Le seuil Prudent doit être ≤ au seuil Dynamique.".into());
    }
    if !(h.precaution_mois_min <= h.precaution_mois_defaut && h.precaution_mois_defaut <= h.precaution_mois_max)
        || h.precaution_mois_min < 0.0
    {
        e.push("Précaution : min ≤ défaut ≤ max (en mois).".into());
    }
    for (nom, v) in [
        ("Frais public", h.frais_scolarite_public),
        ("Frais privé", h.frais_scolarite_prive),
        ("Frais mixte", h.frais_scolarite_mixte),
        ("Coût logement", h.cout_logement_annuel),
        ("Plafond PEA", h.plafond_pea),
        ("Abattement AV", h.abattement_av_seul),
        ("Abattement AV couple", h.abattement_av_couple),
        ("Abattement donation", h.abattement_donation),
    ] {
        montant(&mut e, nom, v);
    }
    if h.glide_path.is_empty() || !h.glide_path.iter().any(|p| p.horizon_min_ans == 0) {
        e.push("Le glide path doit contenir un palier à 0 an.".into());
    }
    for p in &h.glide_path {
        fraction(&mut e, &format!("Glide path ≥ {} ans", p.horizon_min_ans), p.part_actions);
    }
    for (nom, p) in [("Prudent", &h.prudent), ("Équilibré", &h.equilibre), ("Dynamique", &h.dynamique)] {
        let t = p.allocation.total();
        if (t - 1.0).abs() > 0.001 {
            e.push(format!(
                "Profil {nom} : la somme des lignes fait {} % (100 % attendu).",
                format!("{:.1}", t * 100.0).replace('.', ",")
            ));
        }
        for l in Ligne::TOUTES {
            fraction(&mut e, &format!("Profil {nom} / {}", l.libelle()), p.allocation.get(l));
        }
        if !(-1.0..=1.0).contains(&p.rendement) {
            e.push(format!("Profil {nom} : rendement invalide."));
        }
    }

    if e.is_empty() {
        Ok(())
    } else {
        Err(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exemple_valide() {
        assert!(valider(&Questionnaire::exemple(), &Hypotheses::default()).is_ok());
    }

    #[test]
    fn erreurs_detectees() {
        let mut q = Questionnaire::exemple();
        q.adultes[0].tol_risque = 9;
        q.projets[0].montant = -1.0;
        let mut h = Hypotheses::default();
        h.equilibre.allocation.etf_monde = 0.9;
        let e = valider(&q, &h).unwrap_err();
        assert_eq!(e.len(), 3, "{e:?}");
    }
}
