//! Formules financières (capitalisation mensuelle).

/// Valeur future d'un capital `pv` et de versements mensuels `pmt` (fin de mois).
pub fn fv(pv: f64, pmt: f64, taux_annuel: f64, mois: u32) -> f64 {
    let i = taux_annuel / 12.0;
    let n = mois as f64;
    if i.abs() < 1e-12 {
        return pv + pmt * n;
    }
    let f = (1.0 + i).powf(n);
    pv * f + pmt * (f - 1.0) / i
}

/// Versement mensuel nécessaire pour atteindre `cible` en `annees`
/// à partir d'un capital `pv` (formule PMT de la spécification). Jamais négatif.
///
/// Horizon nul : renvoie le manque immédiat (non mensualisé) à charge de l'appelant.
pub fn pmt(cible: f64, pv: f64, taux_annuel: f64, annees: u32) -> f64 {
    let n = 12 * annees;
    if n == 0 {
        return (cible - pv).max(0.0);
    }
    let i = taux_annuel / 12.0;
    let nf = n as f64;
    let v = if i.abs() < 1e-12 {
        (cible - pv) / nf
    } else {
        let f = (1.0 + i).powf(nf);
        (cible - pv * f) * i / (f - 1.0)
    };
    v.max(0.0)
}

/// Plus petit nombre d'années (≤ `max_annees`) pour atteindre `cible`.
pub fn annees_pour_atteindre(
    cible: f64,
    pv: f64,
    pmt: f64,
    taux_annuel: f64,
    max_annees: u32,
) -> Option<u32> {
    (0..=max_annees).find(|a| fv(pv, pmt, taux_annuel, a * 12) >= cible)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pmt_puis_fv_retombe_sur_la_cible() {
        let m = pmt(100_000.0, 10_000.0, 0.05, 10);
        let v = fv(10_000.0, m, 0.05, 120);
        assert!((v - 100_000.0).abs() < 0.01, "{v}");
    }

    #[test]
    fn pmt_taux_nul() {
        assert!((pmt(12_000.0, 0.0, 0.0, 1) - 1_000.0).abs() < 1e-9);
    }

    #[test]
    fn pmt_capital_suffisant() {
        assert_eq!(pmt(10_000.0, 50_000.0, 0.03, 5), 0.0);
    }

    #[test]
    fn horizon() {
        assert_eq!(annees_pour_atteindre(1_000.0, 1_000.0, 0.0, 0.05, 50), Some(0));
        assert_eq!(annees_pour_atteindre(1e12, 0.0, 1.0, 0.0, 50), None);
    }
}
