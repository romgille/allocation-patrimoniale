//! Poche 2 — revenus passifs / long terme : profil, modificateurs, garde-fous,
//! répartition des apports (rééquilibrage par les flux).

use crate::model::{Questionnaire, StatutRp, TempsGestion};
use crate::output::DetailScore;
use crate::params::{Allocation, Hypotheses, Ligne, Profil};

const EPS: f64 = 1e-9;

pub fn score(q: &Questionnaire, h: &Hypotheses) -> (f64, Vec<DetailScore>) {
    let f = q.foyer();
    let tol_libelle = if f.en_couple && f.tol_max != f.tol_risque {
        format!("Tolérance au risque ({}/5, la plus prudente du foyer)", f.tol_risque)
    } else {
        format!("Tolérance au risque ({}/5)", f.tol_risque)
    };
    let mut d = vec![DetailScore { libelle: tol_libelle, points: f.tol_risque as f64 }];
    let hf = if q.h_fire > h.h_fire_long {
        1.0
    } else if q.h_fire < h.h_fire_court {
        -1.0
    } else {
        0.0
    };
    d.push(DetailScore { libelle: format!("Horizon revenus passifs ({} ans)", q.h_fire), points: hf });
    let st = if f.stab_ponderee >= 2.5 {
        1.0
    } else if f.stab_ponderee <= 1.5 {
        -1.0
    } else {
        0.0
    };
    d.push(DetailScore {
        libelle: format!(
            "Stabilité des revenus ({}/3, pondérée par les revenus)",
            format!("{:.1}", f.stab_ponderee).replace('.', ",")
        ),
        points: st,
    });
    let rp = if q.rp == StatutRp::ProprietairePayee { 0.5 } else { 0.0 };
    d.push(DetailScore { libelle: "Résidence principale payée".into(), points: rp });
    let age = if f.age < 40 {
        0.5
    } else if f.age > 55 {
        -1.0
    } else {
        0.0
    };
    d.push(DetailScore {
        libelle: if f.en_couple {
            format!("Âge de l'adulte le plus âgé ({} ans)", f.age)
        } else {
            format!("Âge ({} ans)", f.age)
        },
        points: age,
    });
    (d.iter().map(|x| x.points).sum(), d)
}

pub fn profil_depuis_score(s: f64, h: &Hypotheses) -> Profil {
    if s <= h.seuil_score_prudent {
        Profil::Prudent
    } else if s <= h.seuil_score_dynamique {
        Profil::Equilibre
    } else {
        Profil::Dynamique
    }
}

/// Allocation actuelle de la poche 2, en euros.
pub fn allocation_actuelle_eur(q: &Questionnaire) -> Allocation {
    let a = &q.avoirs;
    Allocation {
        etf_monde: a.etf_monde,
        fonds_euros_obligations: a.fonds_euros_obligations,
        immobilier: q.v_immo_loc + a.scpi,
        or: a.or,
        actions_directes: a.actions_directes,
        crowdfunding: a.crowdfunding_immo + a.crowdfunding_enr,
        crypto: a.crypto,
        private_equity: a.private_equity,
    }
}

#[derive(Debug, Clone)]
pub struct Contexte {
    pub base_eur: f64,
    pub actuelle_pct: Allocation,
    /// Part immobilière totale actuelle (locatif + SCPI + crowdfunding immo) dans la base.
    pub immo_total_pct: f64,
    pub concentration_immo: bool,
}

pub fn contexte(q: &Questionnaire, h: &Hypotheses) -> Contexte {
    let act = allocation_actuelle_eur(q);
    let base = act.total();
    let actuelle_pct = if base > 0.0 { act.map(|_, v| v / base) } else { Allocation::default() };
    let immo_total_pct = if base > 0.0 {
        (act.immobilier + q.avoirs.crowdfunding_immo) / base
    } else {
        0.0
    };
    let denom = q.p_fin() + q.v_immo_loc;
    let ratio = if denom > 0.0 { q.v_immo_loc / denom } else { 0.0 };
    Contexte {
        base_eur: base,
        actuelle_pct,
        immo_total_pct,
        concentration_immo: ratio > h.seuil_concentration_immo,
    }
}

#[derive(Debug, Clone)]
pub struct Cible {
    pub profil: Profil,
    pub allocation: Allocation,
    pub gelees: Vec<Ligne>,
    pub immo_fige: bool,
    pub ajustements: Vec<String>,
    pub plafond_actions: f64,
    pub plafond_immo: f64,
}

fn pct(v: f64) -> String {
    format!("{:.1} %", v * 100.0).replace('.', ",")
}

pub fn allocation_cible(q: &Questionnaire, h: &Hypotheses, ctx: &Contexte, profil_score: Profil) -> Cible {
    let f = q.foyer();
    let mut aj = Vec::new();
    let mut profil = profil_score;

    // H_fire court : descendre d'un profil.
    let horizon_court = q.h_fire < h.h_fire_court;
    if horizon_court && profil != Profil::Prudent {
        let p = profil.inferieur();
        aj.push(format!(
            "Horizon < {} ans : profil abaissé de {} à {}.",
            h.h_fire_court,
            profil.libelle(),
            p.libelle()
        ));
        profil = p;
    }
    let mut a = h.profil(profil).allocation;

    // Crypto.
    let c = &q.contraintes;
    if c.refus_crypto {
        a.crypto = 0.0;
    } else if c.crypto_souhaitee_pct > 0.0 {
        let voulu = (c.crypto_souhaitee_pct / 100.0).min(h.crypto_max).min(a.actions_directes);
        if voulu > 0.0 {
            a.crypto = voulu;
            a.actions_directes -= voulu;
            aj.push(format!("Crypto : {} prélevés sur la ligne actions directes.", pct(voulu)));
        } else {
            aj.push("Crypto souhaitée mais aucune marge sur la ligne actions directes : 0 %.".into());
        }
    }

    // Plafond immobilier.
    let plafond_immo = if f.temps_gestion == TempsGestion::Eleve {
        h.immo_max_gestion_elevee
    } else {
        h.immo_max
    };

    // Concentration immobilière : on fige l'immobilier à sa valeur existante.
    let immo_fige = ctx.concentration_immo;
    if immo_fige {
        a.immobilier = ctx.actuelle_pct.immobilier;
        aj.push(format!(
            "Immobilier locatif > {} du patrimoine : cible immobilier = part existante ({}), \
             aucun nouveau flux immobilier (SCPI, crowdfunding immo compris). \
             Envisager de vendre / arbitrer.",
            pct(h.seuil_concentration_immo),
            pct(a.immobilier)
        ));
    }

    if f.temps_gestion == TempsGestion::Faible || c.refus_immo_direct {
        aj.push("Immobilier uniquement via SCPI (en AV/PER si TMI ≥ 30 %) ou ETF immobilier coté : pas de locatif direct.".into());
    }

    // Plafonds actions.
    let mut plafond_actions = (h.age_regle_actions.saturating_sub(f.age)) as f64 / 100.0;
    if f.tol_risque <= 2 {
        plafond_actions = plafond_actions.min(h.actions_max_tolerance_faible);
    }
    plafond_actions = plafond_actions.clamp(0.0, 1.0);

    let plancher_fe = if horizon_court {
        h.plancher_fonds_euros_horizon_court
    } else {
        h.plancher_fonds_euros
    };
    if horizon_court {
        aj.push(format!("Horizon court : fonds euros porté à au moins {}.", pct(plancher_fe)));
    }

    let avant = a;
    let a = appliquer_garde_fous(a, h, plafond_actions, plancher_fe, plafond_immo, immo_fige);

    for l in Ligne::TOUTES {
        let (x, y) = (avant.get(l), a.get(l));
        if (x - y).abs() > 0.0005 {
            aj.push(format!("Garde-fous : {} {} → {}.", l.libelle(), pct(x), pct(y)));
        }
    }
    if (a.actions() - plafond_actions).abs() < 0.0005 && avant.actions() > plafond_actions + 0.0005 {
        aj.push(format!(
            "Part actions plafonnée à {} (règle {} − âge{}).",
            pct(plafond_actions),
            h.age_regle_actions,
            if f.tol_risque <= 2 { ", tolérance faible" } else { "" }
        ));
    }

    // Lignes fermées aux nouveaux apports.
    let mut gelees = Vec::new();
    for l in Ligne::TOUTES {
        if a.get(l) <= EPS {
            gelees.push(l);
        }
    }
    let immo_au_plafond = ctx.immo_total_pct > plafond_immo + EPS;
    if (immo_fige || immo_au_plafond) && !gelees.contains(&Ligne::Immobilier) {
        gelees.push(Ligne::Immobilier);
        if immo_au_plafond && !immo_fige {
            aj.push(format!(
                "Immobilier total actuel {} > plafond {} : ligne immobilier fermée aux apports.",
                pct(ctx.immo_total_pct),
                pct(plafond_immo)
            ));
        }
    }
    if ctx.actuelle_pct.crowdfunding > h.crowdfunding_max + EPS && !gelees.contains(&Ligne::Crowdfunding) {
        gelees.push(Ligne::Crowdfunding);
        aj.push(format!(
            "Crowdfunding actuel {} > {} : ligne fermée aux apports, ne pas réinvestir les remboursements.",
            pct(ctx.actuelle_pct.crowdfunding),
            pct(h.crowdfunding_max)
        ));
    }

    Cible { profil, allocation: a, gelees, immo_fige, ajustements: aj, plafond_actions, plafond_immo }
}

/// Applique plafonds / planchers puis renormalise à 100 %.
/// Les résidus sont absorbés par l'ETF monde (dans la limite du plafond actions),
/// puis par les fonds euros.
pub fn appliquer_garde_fous(
    mut a: Allocation,
    h: &Hypotheses,
    plafond_actions: f64,
    plancher_fe: f64,
    plafond_immo: f64,
    immo_fige: bool,
) -> Allocation {
    let immo_fixe_val = a.immobilier;
    for _ in 0..20 {
        a = a.map(|_, v| v.max(0.0));
        a.or = a.or.clamp(h.or_min, h.or_max);
        a.crowdfunding = a.crowdfunding.min(h.crowdfunding_max);
        a.actions_directes = a.actions_directes.min(h.actions_directes_max);
        a.crypto = a.crypto.min(h.crypto_max);

        let mut exces = a.satellites() - h.satellites_max;
        for l in [Ligne::Crypto, Ligne::PrivateEquity, Ligne::ActionsDirectes, Ligne::Crowdfunding] {
            if exces <= EPS {
                break;
            }
            let v = a.get(l);
            let r = v.min(exces);
            a.set(l, v - r);
            exces -= r;
        }

        if immo_fige {
            a.immobilier = immo_fixe_val;
        } else {
            // Le crowdfunding est compté (prudemment) comme immobilier pour le plafond.
            a.immobilier = a.immobilier.min((plafond_immo - a.crowdfunding).max(0.0));
        }

        let act = a.actions();
        if act > plafond_actions + EPS {
            let k = plafond_actions / act;
            a.etf_monde *= k;
            a.actions_directes *= k;
        }
        a.fonds_euros_obligations = a.fonds_euros_obligations.max(plancher_fe);

        let mut residu = 1.0 - a.total();
        if residu.abs() <= EPS {
            break;
        }
        if residu > 0.0 {
            let marge = (plafond_actions - a.actions()).max(0.0);
            let x = residu.min(marge);
            a.etf_monde += x;
            residu -= x;
            a.fonds_euros_obligations += residu;
        } else {
            // Trop d'allocation : on retire dans l'ordre ETF > fonds euros (au plancher)
            // > immobilier (si non figé) > or (au plancher) > satellites.
            let mut trop = -residu;
            let ordre: [(Ligne, f64); 7] = [
                (Ligne::EtfMonde, 0.0),
                (Ligne::FondsEurosObligations, plancher_fe),
                (Ligne::Immobilier, if immo_fige { f64::INFINITY } else { 0.0 }),
                (Ligne::Crowdfunding, 0.0),
                (Ligne::ActionsDirectes, 0.0),
                (Ligne::Or, h.or_min),
                (Ligne::FondsEurosObligations, 0.0),
            ];
            for (l, plancher) in ordre {
                if trop <= EPS {
                    break;
                }
                let v = a.get(l);
                let dispo = (v - plancher).max(0.0);
                let r = dispo.min(trop);
                a.set(l, v - r);
                trop -= r;
            }
            if trop > EPS {
                // Cas extrême (immobilier figé > 100 % − planchers) : on renormalise.
                return a.normalisee();
            }
        }
    }
    a
}

/// Répartit `montant` entre les lignes non gelées pour réduire d'abord les écarts
/// les plus négatifs (remplissage « par le bas »), puis au prorata des cibles.
pub fn repartir(montant: f64, actuel: &Allocation, cible: &Allocation, gelees: &[Ligne]) -> Allocation {
    let mut out = Allocation::default();
    if montant <= 0.0 {
        return out;
    }
    let ouvertes: Vec<Ligne> = Ligne::TOUTES.into_iter().filter(|l| !gelees.contains(l)).collect();
    if ouvertes.is_empty() {
        out.fonds_euros_obligations = montant;
        return out;
    }
    let total_apres = actuel.total() + montant;
    let deficits: Vec<(Ligne, f64)> = ouvertes
        .iter()
        .map(|l| (*l, (cible.get(*l) * total_apres - actuel.get(*l)).max(0.0)))
        .collect();
    let somme: f64 = deficits.iter().map(|x| x.1).sum();

    if somme <= montant {
        for (l, d) in &deficits {
            out.set(*l, *d);
        }
        let reste = montant - somme;
        let poids: f64 = ouvertes.iter().map(|l| cible.get(*l)).sum();
        for l in &ouvertes {
            let w = if poids > 0.0 { cible.get(*l) / poids } else { 1.0 / ouvertes.len() as f64 };
            *out.get_mut(*l) += reste * w;
        }
    } else {
        // Trouver le niveau L tel que Σ max(0, d − L) = montant.
        let (mut lo, mut hi) = (0.0_f64, deficits.iter().map(|x| x.1).fold(0.0, f64::max));
        for _ in 0..100 {
            let mid = (lo + hi) / 2.0;
            let s: f64 = deficits.iter().map(|x| (x.1 - mid).max(0.0)).sum();
            if s > montant {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        for (l, d) in &deficits {
            out.set(*l, (d - hi).max(0.0));
        }
        // Correction d'arrondi.
        let t = out.total();
        if t > 0.0 {
            out = out.map(|_, v| v * montant / t);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Questionnaire;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn score_exemple() {
        let q = Questionnaire::exemple();
        let h = Hypotheses::default();
        let (s, _) = score(&q, &h);
        // tolérance la plus prudente 3 + 0 (15 ans) + 1 (stab pondérée 2,55) + 0 + 0.5 (36 ans)
        assert!(approx(s, 4.5));
        assert_eq!(profil_depuis_score(s, &h), Profil::Equilibre);
        assert_eq!(profil_depuis_score(3.0, &h), Profil::Prudent);
        assert_eq!(profil_depuis_score(5.0, &h), Profil::Equilibre);
        assert_eq!(profil_depuis_score(5.5, &h), Profil::Dynamique);
    }

    #[test]
    fn profils_par_defaut_inchanges_par_garde_fous() {
        let h = Hypotheses::default();
        for p in [Profil::Prudent, Profil::Equilibre, Profil::Dynamique] {
            let a = h.profil(p).allocation;
            assert!(approx(a.total(), 1.0));
            let b = appliquer_garde_fous(a, &h, 0.75, 0.10, 0.40, false);
            for l in Ligne::TOUTES {
                assert!(approx(a.get(l), b.get(l)), "{p:?} {l:?}");
            }
        }
    }

    #[test]
    fn plafond_actions_age() {
        let h = Hypotheses::default();
        let a = h.dynamique.allocation;
        let b = appliquer_garde_fous(a, &h, 0.50, 0.10, 0.40, false);
        assert!(approx(b.total(), 1.0));
        assert!(b.actions() <= 0.50 + 1e-9);
        assert!(b.fonds_euros_obligations > a.fonds_euros_obligations);
    }

    #[test]
    fn immo_fige_eleve() {
        let h = Hypotheses::default();
        let mut a = h.equilibre.allocation;
        a.immobilier = 0.70;
        let b = appliquer_garde_fous(a, &h, 0.75, 0.10, 0.40, true);
        assert!(approx(b.total(), 1.0));
        assert!(approx(b.immobilier, 0.70));
        assert!(b.fonds_euros_obligations >= 0.10 - 1e-9);
        assert!(b.or >= 0.05 - 1e-9);
    }

    #[test]
    fn horizon_court_descend_et_plancher() {
        let mut q = Questionnaire::exemple();
        q.h_fire = 5;
        for a in &mut q.adultes {
            a.tol_risque = 5;
        }
        let h = Hypotheses::default();
        let ctx = contexte(&q, &h);
        let c = allocation_cible(&q, &h, &ctx, Profil::Dynamique);
        assert_eq!(c.profil, Profil::Equilibre);
        assert!(c.allocation.fonds_euros_obligations >= 0.25 - 1e-9);
        assert!(approx(c.allocation.total(), 1.0));
    }

    #[test]
    fn crypto_prise_sur_actions_directes() {
        let mut q = Questionnaire::exemple();
        q.contraintes.crypto_souhaitee_pct = 10.0;
        let h = Hypotheses::default();
        let ctx = contexte(&q, &h);
        let c = allocation_cible(&q, &h, &ctx, Profil::Equilibre);
        assert!(approx(c.allocation.crypto, 0.03));
        assert!(approx(c.allocation.actions_directes, 0.02));
        q.contraintes.refus_crypto = true;
        let c = allocation_cible(&q, &h, &ctx, Profil::Equilibre);
        assert_eq!(c.allocation.crypto, 0.0);
        assert!(c.gelees.contains(&Ligne::Crypto));
    }

    #[test]
    fn concentration_immo() {
        let mut q = Questionnaire::exemple();
        q.v_immo_loc = 400_000.0;
        let h = Hypotheses::default();
        let ctx = contexte(&q, &h);
        assert!(ctx.concentration_immo);
        let c = allocation_cible(&q, &h, &ctx, Profil::Equilibre);
        assert!(c.immo_fige);
        assert!(c.gelees.contains(&Ligne::Immobilier));
        assert!(approx(c.allocation.immobilier, ctx.actuelle_pct.immobilier));
        assert!(approx(c.allocation.total(), 1.0));
    }

    #[test]
    fn repartition_remplit_les_ecarts() {
        let cible = Allocation { etf_monde: 0.5, fonds_euros_obligations: 0.5, ..Default::default() };
        let actuel = Allocation { etf_monde: 100.0, fonds_euros_obligations: 0.0, ..Default::default() };
        let r = repartir(50.0, &actuel, &cible, &[Ligne::Immobilier]);
        assert!(approx(r.fonds_euros_obligations, 50.0));
        assert!(approx(r.etf_monde, 0.0));
        // Surplus au-delà des écarts : au prorata des cibles.
        let r = repartir(300.0, &actuel, &cible, &[]);
        assert!(approx(r.total(), 300.0));
        assert!(approx(actuel.etf_monde + r.etf_monde, 200.0));
        assert!(approx(r.fonds_euros_obligations, 200.0));
    }

    #[test]
    fn repartition_niveau_egalise() {
        let cible = Allocation { etf_monde: 0.5, fonds_euros_obligations: 0.3, or: 0.2, ..Default::default() };
        let actuel = Allocation::default();
        let r = repartir(100.0, &actuel, &cible, &[]);
        assert!(approx(r.etf_monde, 50.0) && approx(r.or, 20.0));
        let gel = repartir(100.0, &actuel, &cible, &[Ligne::Or]);
        assert!(approx(gel.or, 0.0) && approx(gel.total(), 100.0));
    }
}
