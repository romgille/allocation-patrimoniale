//! Mathématiques d'un crédit amortissable à mensualités constantes.
//!
//! Le questionnaire demande le taux, le restant dû, la mensualité et la durée restante.
//! Ces quatre valeurs sont liées : on recalcule donc la mensualité théorique pour signaler
//! les saisies incohérentes, et on déroule l'amortissement pour connaître la part d'intérêts
//! (déductible dans une SCI) et le moment où la mensualité se libère.

/// Mensualité d'un prêt amortissable : capital `restant_du`, taux annuel `taux`
/// (0,013 pour 1,3 %), sur `mois` mensualités. Taux nul : simple division.
pub fn mensualite_theorique(restant_du: f64, taux: f64, mois: u32) -> f64 {
    if mois == 0 || restant_du <= 0.0 {
        return 0.0;
    }
    let i = taux / 12.0;
    if i.abs() < 1e-12 {
        return restant_du / mois as f64;
    }
    let f = (1.0 + i).powi(mois as i32);
    restant_du * i * f / (f - 1.0)
}

/// Intérêts restant à payer jusqu'à la fin du prêt (mensualités × durée − capital).
pub fn interets_restants(restant_du: f64, taux: f64, mois: u32, mensualite: f64) -> f64 {
    if mois == 0 || restant_du <= 0.0 {
        return 0.0;
    }
    // On déroule l'amortissement : la mensualité saisie peut être incohérente, et une
    // simple soustraction donnerait alors un résultat absurde.
    let i = taux / 12.0;
    let mut capital = restant_du;
    let mut total = 0.0;
    for _ in 0..mois {
        let interet = (capital * i).max(0.0);
        total += interet;
        capital = (capital + interet - mensualite).max(0.0);
        if capital <= 0.0 {
            break;
        }
    }
    total
}

/// Intérêts payés sur les `mois` prochaines mensualités (12 = année en cours).
/// Sert au résultat fiscal d'une SCI, où seuls les intérêts sont déductibles.
pub fn interets_sur(restant_du: f64, taux: f64, mensualite: f64, mois: u32) -> f64 {
    let i = taux / 12.0;
    let mut capital = restant_du;
    let mut total = 0.0;
    for _ in 0..mois {
        if capital <= 0.0 {
            break;
        }
        let interet = (capital * i).max(0.0);
        total += interet;
        capital = (capital + interet - mensualite).max(0.0);
    }
    total
}

/// Durée réelle d'extinction avec la mensualité saisie, plafonnée à `max_mois`.
/// `None` si la mensualité ne couvre même pas les intérêts (le capital ne baisse jamais).
pub fn duree_implicite(restant_du: f64, taux: f64, mensualite: f64, max_mois: u32) -> Option<u32> {
    if restant_du <= 0.0 {
        return Some(0);
    }
    if mensualite <= 0.0 {
        return None;
    }
    let i = taux / 12.0;
    let mut capital = restant_du;
    for m in 0..max_mois {
        let interet = (capital * i).max(0.0);
        capital = capital + interet - mensualite;
        if capital <= 0.0 {
            return Some(m + 1);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn mensualite_de_reference() {
        // 200 000 € à 1,30 % sur 20 ans : mensualité de référence ≈ 946,80 €.
        assert!(approx(mensualite_theorique(200_000.0, 0.013, 240), 946.80, 0.5));
        // Taux nul : le capital divisé par la durée.
        assert!(approx(mensualite_theorique(12_000.0, 0.0, 24), 500.0, 1e-9));
        assert_eq!(mensualite_theorique(10_000.0, 0.02, 0), 0.0);
        assert_eq!(mensualite_theorique(0.0, 0.02, 120), 0.0);
    }

    #[test]
    fn interets_coherents_avec_la_mensualite() {
        let (du, taux, mois) = (200_000.0, 0.013, 240);
        let m = mensualite_theorique(du, taux, mois);
        // Prêt cohérent : intérêts = total versé − capital.
        assert!(approx(interets_restants(du, taux, mois, m), m * mois as f64 - du, 1.0));
        // Les intérêts de la première année restent sous capital × taux (le capital baisse).
        let an1 = interets_sur(du, taux, m, 12);
        assert!(an1 < du * taux && an1 > du * taux * 0.9, "{an1}");
        assert_eq!(interets_restants(0.0, 0.05, 120, 100.0), 0.0);
    }

    #[test]
    fn duree_implicite_et_mensualite_insuffisante() {
        let (du, taux) = (50_000.0, 0.03);
        let m = mensualite_theorique(du, taux, 120);
        assert_eq!(duree_implicite(du, taux, m, 600), Some(120));
        // Mensualité inférieure aux intérêts mensuels : le prêt ne s'éteint jamais.
        assert_eq!(duree_implicite(du, taux, 50.0, 600), None);
        assert_eq!(duree_implicite(0.0, 0.03, 0.0, 600), Some(0));
    }
}
