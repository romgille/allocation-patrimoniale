//! Moteur d'allocation patrimoniale — France, double objectif
//! « études des enfants » + « revenus passifs ».
//!
//! Point d'entrée : [`calculer`]. Informatif et pédagogique, pas un conseil en investissement.

pub mod cession;
pub mod credit;
pub mod finance;
pub mod model;
pub mod output;
pub mod params;
pub mod poche1;
pub mod poche2;
pub mod sci;
mod validation;

pub use model::*;
pub use output::*;
pub use params::*;
pub use validation::valider;

use finance::{fv, pmt};
use poche2::{allocation_actuelle_eur, allocation_cible, contexte, profil_depuis_score, repartir, score};

fn pct(v: f64) -> String {
    format!("{:.1} %", v * 100.0).replace('.', ",")
}

/// Formatage partagé avec les modules `sci` et `cession`.
pub(crate) fn pct_public(v: f64) -> String {
    pct(v)
}

pub(crate) fn eur_public(v: f64) -> String {
    eur(v)
}

fn eur(v: f64) -> String {
    let s = format!("{:.0}", v.round().abs());
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push('\u{202f}');
        }
        out.push(c);
    }
    format!("{}{} €", if v < -0.5 { "-" } else { "" }, out)
}

fn enveloppe_et_support(
    l: Ligne,
    q: &Questionnaire,
    h: &Hypotheses,
    immo_fige: bool,
) -> (String, String) {
    let tmi_haute = q.tmi.haute();
    let tmi_basse = q.tmi.basse();
    let pea_utilise = q.avoirs.etf_monde + q.avoirs.actions_directes;
    match l {
        Ligne::EtfMonde => (
            if pea_utilise >= h.plafond_pea {
                "PEA plein : assurance-vie, puis CTO".into()
            } else {
                format!("PEA (jusqu'à {} de versements), puis AV, puis CTO", eur(h.plafond_pea))
            },
            if q.contraintes.esg {
                "1 ETF MSCI World SRI / ESG capitalisant (version éligible PEA)".into()
            } else {
                "1 ETF MSCI World (ou ACWI) capitalisant, version éligible PEA — pas de doublon S&P 500".into()
            },
        ),
        Ligne::FondsEurosObligations => (
            if tmi_basse {
                "Assurance-vie (fonds euros) ; ETF obligataire court terme possible".into()
            } else {
                "Assurance-vie (fonds euros) ; ETF monétaire pour le cash du PEA".into()
            },
            "Fonds euros (frais de gestion ≤ 0,6 %) ; ETF obligataire euro court terme".into(),
        ),
        Ligne::Immobilier => {
            if immo_fige {
                (
                    "Aucun nouvel apport".into(),
                    "Conserver, vendre ou arbitrer l'existant (poids immobilier trop élevé)".into(),
                )
            } else {
                let env = if tmi_haute {
                    "SCPI en assurance-vie ou PER — jamais en direct (TMI ≥ 30 %)".to_string()
                } else if tmi_basse {
                    "SCPI en direct possible (TMI ≤ 11 %) ou en AV".to_string()
                } else {
                    "SCPI en assurance-vie de préférence".to_string()
                };
                let sup = if q.foyer(h).temps_gestion == TempsGestion::Faible || q.contraintes.refus_immo_direct {
                    "2–3 SCPI diversifiées/européennes sans frais d'entrée, ou ETF immobilier coté"
                } else {
                    "SCPI ; locatif direct uniquement financé à crédit (levier)"
                };
                (env, sup.into())
            }
        }
        Ligne::Or => ("CTO (parfois disponible en AV)".into(), "ETC or physique".into()),
        Ligne::ActionsDirectes => (
            "PEA".into(),
            "≤ 10 lignes, grandes capitalisations — budget « erreur acceptable »".into(),
        ),
        Ligne::Crowdfunding => (
            format!("Direct (PFU {})", pct(h.pfu)),
            if immo_fige {
                "Énergies renouvelables uniquement (Enerfip, Lendosphère…)".into()
            } else {
                format!(
                    "Plateformes agréées PSFP : ≥ 20 projets, ≥ 3 plateformes, ≤ {} par projet, mixer immo (avec sûretés) et ENR",
                    eur(q.p_fin() * 0.01)
                )
            },
        ),
        Ligne::Crypto => ("Plateforme régulée (MiCA) / CTO".into(), "BTC/ETH — satellite ≤ 3 %".into()),
        Ligne::PrivateEquity => ("—".into(), "Non recommandé par la stratégie (satellite)".into()),
    }
}

/// Simulation mensuelle de la poche 2 : l'épargne comble d'abord la précaution et les
/// dettes chères, puis finance les projets en cours (par priorité) ; le reste va en poche 2.
///
/// L'épargne disponible n'est pas constante dans le temps :
/// - quand un projet arrive à échéance, son versement est libéré ;
/// - quand un crédit du foyer se termine, sa mensualité revient à l'épargne ;
/// - quand une SCI cesse d'être en trésorerie négative, l'effort du foyer disparaît ;
/// - une vente programmée verse son produit net en une fois.
struct Simulation<'a> {
    capital: f64,
    e_mois: f64,
    a_combler: f64,
    poches: &'a [SousPoche],
    rendement: f64,
    /// Mensualités du foyer qui se libèrent : (mois de fin, montant).
    liberations: Vec<(u32, f64)>,
    /// Produits nets des ventes : (mois, montant), déjà nets d'impôt et de crédit soldé.
    apports: Vec<(u32, f64)>,
}

impl Simulation<'_> {
    /// Épargne mensuelle disponible au mois `mois`.
    fn e_mois_a(&self, mois: u32) -> f64 {
        let liberees: f64 =
            self.liberations.iter().filter(|(fin, _)| mois >= *fin).map(|(_, m)| m).fold(0.0, |a, b| a + b);
        (self.e_mois + liberees).max(0.0)
    }

    fn flux_poche2(&self, mois: u32, a_combler: &mut f64) -> f64 {
        let mut e = self.e_mois_a(mois);
        let c = e.min(*a_combler);
        *a_combler -= c;
        e -= c;
        let actives: Vec<SousPoche> =
            self.poches.iter().filter(|p| mois < p.horizon_ans * 12).cloned().collect();
        let projets: f64 = poche1::repartir_par_priorite(e, &actives).iter().fold(0.0, |a, b| a + b);
        (e - projets).max(0.0)
    }

    /// Produit des ventes tombant exactement au mois `mois`.
    fn apport_a(&self, mois: u32) -> f64 {
        self.apports.iter().filter(|(m, _)| *m == mois).map(|(_, v)| v).fold(0.0, |a, b| a + b)
    }

    fn capital_a(&self, mois: u32) -> f64 {
        let i = self.rendement / 12.0;
        let mut k = self.capital;
        let mut a_combler = self.a_combler;
        for m in 0..mois {
            k = k * (1.0 + i) + self.flux_poche2(m, &mut a_combler) + self.apport_a(m);
        }
        k += self.apport_a(mois);
        k
    }

    fn flux_poche2_cumule(&self, mois: u32) -> f64 {
        let mut a_combler = self.a_combler;
        (0..mois).map(|m| self.flux_poche2(m, &mut a_combler)).fold(0.0, |a, b| a + b)
    }
}

/// Lecture de chaque crédit : coût réel, cohérence de la saisie et arbitrage entre
/// rembourser par anticipation et investir. Renvoie les analyses, le capital des crédits
/// à rembourser en priorité, et les alertes correspondantes.
fn analyser_credits(
    q: &Questionnaire,
    h: &Hypotheses,
    rendement_net: f64,
) -> (Vec<AnalyseCredit>, f64, Vec<Alerte>) {
    let mut alertes = Vec::new();
    let mut du_prioritaire = 0.0;
    let mut credits = Vec::new();

    for d in &q.dettes {
        let taux = d.taux();
        let porte_par = match &d.sci_id {
            None => "Foyer".to_string(),
            Some(id) => q
                .scis
                .iter()
                .find(|s| &s.id == id)
                .map(|s| s.nom.clone())
                .unwrap_or_else(|| "SCI inconnue".into()),
        };

        // Cohérence : avec ce taux, ce capital et cette durée, la mensualité devrait être…
        let theorique = credit::mensualite_theorique(d.restant_du, taux, d.duree_restante_mois);
        let coherent = d.duree_restante_mois == 0
            || theorique <= 0.0
            || ((d.mensualite - theorique).abs() / theorique <= h.ecart_mensualite_max);
        if !coherent {
            alertes.push(Alerte::attention(format!(
                "Crédit « {} » : avec {} % sur {} mois pour {}, la mensualité devrait être d'environ {} et non {}. Vérifier la saisie (assurance emprunteur comprise ou non, différé…).",
                d.libelle,
                format!("{:.2}", d.taux_pct).replace('.', ","),
                d.duree_restante_mois,
                eur(d.restant_du),
                eur(theorique),
                eur(d.mensualite)
            )));
        }

        let interets = credit::interets_restants(d.restant_du, taux, d.duree_restante_mois, d.mensualite);

        // Arbitrage : trois zones, selon le taux face au rendement net attendu.
        let (verdict, commentaire) = if taux > h.seuil_taux_dette {
            if d.sci_id.is_none() {
                du_prioritaire += d.restant_du;
            }
            alertes.push(Alerte::critique(format!(
                "Crédit « {} » à {} % : rembourser en priorité avant d'investir, c'est un rendement garanti supérieur à tout placement raisonnable.",
                d.libelle,
                format!("{:.2}", d.taux_pct).replace('.', ",")
            )));
            (
                VerdictCredit::Rembourser,
                format!(
                    "Taux au-dessus du seuil de {} : le remboursement anticipé rapporte plus, sans risque, que la poche 2.",
                    pct(h.seuil_taux_dette)
                ),
            )
        } else if taux > rendement_net {
            alertes.push(Alerte::attention(format!(
                "Crédit « {} » à {} % : au-dessus du rendement net attendu de la poche 2 ({}). Rembourser par anticipation est un placement sûr ; investir peut rapporter plus, mais sans garantie.",
                d.libelle,
                format!("{:.2}", d.taux_pct).replace('.', ","),
                pct(rendement_net)
            )));
            (
                VerdictCredit::Arbitrer,
                format!(
                    "Entre le rendement net attendu ({}) et le seuil de {} : arbitrage personnel entre sécurité et espérance de gain.",
                    pct(rendement_net),
                    pct(h.seuil_taux_dette)
                ),
            )
        } else if d.objet.productif() {
            (
                VerdictCredit::Levier,
                format!(
                    "Crédit à {} sur un bien qui produit des loyers, en dessous du rendement net attendu ({}) : c'est un levier, le rembourser par anticipation appauvrirait le foyer.",
                    pct(taux),
                    pct(rendement_net)
                ),
            )
        } else {
            (
                VerdictCredit::Conserver,
                format!(
                    "Taux inférieur au rendement net attendu de la poche 2 ({}) : garder le crédit et investir l'épargne.",
                    pct(rendement_net)
                ),
            )
        };

        credits.push(AnalyseCredit {
            libelle: d.libelle.clone(),
            objet_libelle: d.objet.libelle().into(),
            porte_par,
            taux_pct: d.taux_pct,
            restant_du_eur: d.restant_du,
            mensualite_eur: d.mensualite,
            duree_restante_mois: d.duree_restante_mois,
            interets_restants_eur: interets,
            mensualite_theorique_eur: theorique,
            coherent,
            verdict,
            verdict_libelle: verdict.libelle().into(),
            commentaire,
        });
    }

    // Une mensualité qui se libère bientôt mérite d'être anticipée.
    let bientot: Vec<&Dette> = q
        .dettes_foyer()
        .filter(|d| matches!(d.fin_mois(), Some(m) if m <= 24) && d.mensualite > 0.0)
        .collect();
    for d in &bientot {
        alertes.push(Alerte::info(format!(
            "Crédit « {} » soldé dans {} mois : {} par mois reviendront à l'épargne. Les affecter à la poche 2 dès maintenant évite qu'ils se diluent dans les dépenses.",
            d.libelle,
            d.duree_restante_mois,
            eur(d.mensualite)
        )));
    }

    if q.dettes.iter().any(|d| d.duree_restante_mois == 0 && d.mensualite > 0.0) {
        alertes.push(Alerte::info(
            "Certains crédits n'ont pas de durée renseignée : ils sont supposés courir sur toute la projection, ce qui sous-estime l'épargne future."
                .to_string(),
        ));
    }

    (credits, du_prioritaire, alertes)
}

fn budget(q: &Questionnaire, f: &Foyer) -> Budget {
    let part = |m: f64| if f.revenus > 0.0 { m / f.revenus } else { 0.0 };
    let postes = PosteDepense::TOUS
        .iter()
        .filter_map(|p| {
            let m: f64 = q.depenses.iter().filter(|d| d.poste == *p).map(|d| d.montant_mensuel).fold(0.0, |a, b| a + b);
            (m > 0.0).then(|| PosteBudget {
                poste: *p,
                libelle: p.libelle().into(),
                montant_eur: m,
                part_revenus: part(m),
            })
        })
        .collect();
    Budget {
        adultes: q
            .adultes
            .iter()
            .map(|a| RevenuAdulte {
                prenom: a.prenom.clone(),
                montant_eur: a.revenu_total(),
                part: part(a.revenu_total()),
                tol_risque: a.tol_risque,
                stab_revenus: a.stab_revenus,
                temps_gestion: a.temps_gestion,
            })
            .collect(),
        total_revenus_eur: f.revenus,
        postes,
        total_depenses_eur: f.depenses,
        mensualites_credits_eur: f.mensualites,
        capacite_calculee_eur: f.capacite_calculee,
        epargne_retenue_eur: f.e_mois,
        epargne_forcee: q.epargne_mensuelle_forcee.is_some(),
        taux_epargne: part(f.e_mois),
        non_affecte_eur: (f.capacite_calculee - f.e_mois).max(0.0),
    }
}

pub fn calculer(q: &Questionnaire, h: &Hypotheses) -> Resultat {
    let mut alertes: Vec<Alerte> = Vec::new();

    // ---------- Synthèse ----------
    let f = q.foyer(h);
    let p_fin = q.p_fin();
    let immo_net = q.immo_locatif_net(h);
    let denom = p_fin + immo_net;
    let synthese = Synthese {
        nb_adultes: q.adultes.len() as u32,
        nb_enfants: q.enfants.len() as u32,
        age_reference: f.age,
        tol_risque_retenue: f.tol_risque,
        p_fin_eur: p_fin,
        v_immo_loc_eur: immo_net,
        ratio_immo_locatif: if denom > 0.0 { immo_net / denom } else { 0.0 },
        taux_endettement: q.taux_endettement(h),
        taux_endettement_bancaire: q.taux_endettement_bancaire(h),
        tmi_pct: q.tmi.pct(),
    };

    // ---------- Budget du foyer ----------
    let budget = budget(q, &f);
    if f.revenus <= 0.0 {
        alertes.push(Alerte::critique("Aucun revenu saisi pour le foyer.".to_string()));
    } else if f.capacite_calculee < 0.0 {
        alertes.push(Alerte::critique(format!(
            "Budget déficitaire : les dépenses et mensualités dépassent les revenus de {} par mois.",
            eur(-f.capacite_calculee)
        )));
    } else if budget.taux_epargne < 0.10 {
        alertes.push(Alerte::attention(format!(
            "Taux d'épargne de {} : il sera difficile de financer projets et revenus passifs.",
            pct(budget.taux_epargne)
        )));
    }
    if let Some(forcee) = q.epargne_mensuelle_forcee {
        if forcee > f.capacite_calculee + 0.5 {
            alertes.push(Alerte::attention(format!(
                "Épargne imposée ({}) supérieure à la capacité calculée ({}) : vérifier le budget.",
                eur(forcee),
                eur(f.capacite_calculee.max(0.0))
            )));
        }
    }
    if synthese.taux_endettement > h.endettement_max {
        alertes.push(Alerte::attention(format!(
            "Taux d'endettement du foyer de {} (> {}).",
            pct(synthese.taux_endettement),
            pct(h.endettement_max)
        )));
    } else if synthese.taux_endettement_bancaire > h.endettement_max {
        alertes.push(Alerte::attention(format!(
            "Taux d'endettement de {} du foyer seul, mais {} en vue bancaire (crédits de SCI compris, loyers retenus à {}) : c'est ce second chiffre qu'une banque regardera pour un nouveau prêt.",
            pct(synthese.taux_endettement),
            pct(synthese.taux_endettement_bancaire),
            pct(h.ponderation_loyers_bancaire)
        )));
    }
    if f.effort_scis > 0.0 {
        alertes.push(Alerte::attention(format!(
            "Les SCI demandent un effort de {} par mois au foyer : leurs loyers ne couvrent pas leurs charges et leurs crédits. C'est autant d'épargne en moins.",
            eur(f.effort_scis)
        )));
    }
    if f.en_couple && f.tol_max >= f.tol_risque + 2 {
        let noms = |t: u8| {
            q.adultes.iter().filter(|a| a.tol_risque == t).map(|a| a.prenom.as_str()).collect::<Vec<_>>().join(", ")
        };
        alertes.push(Alerte::attention(format!(
            "Rapports au risque très différents dans le foyer ({} : {}/5, {} : {}/5) : le plus prudent est retenu. À discuter ensemble avant d'investir.",
            noms(f.tol_risque),
            f.tol_risque,
            noms(f.tol_max),
            f.tol_max
        )));
    }

    // ---------- Poche 0 ----------
    let p0 = poche1::poche0(q, &f, h);
    if p0.manque_eur > 0.0 {
        alertes.push(Alerte::critique(format!(
            "Épargne de précaution insuffisante : il manque {} (cible {} mois de dépenses). \
             Tout le flux y va d'abord{}.",
            eur(p0.manque_eur),
            p0.cible_mois,
            if f.abondement_employeur { " — sauf l'abondement employeur à ne pas laisser passer" } else { "" }
        )));
    }
    if p0.excedent_eur > 1_000.0 {
        alertes.push(Alerte::info(format!(
            "Excédent de précaution de {} : il est redéployé dans la poche 2.",
            eur(p0.excedent_eur)
        )));
    }

    // ---------- Poche 1 : études et projets ----------
    let p1: Vec<SousPoche> = poche1::sous_poches(q, h);
    for s in &p1 {
        if s.deficit_immediat_eur > 0.0 {
            alertes.push(Alerte::critique(format!(
                "{} : échéance immédiate, il manque {} (capital à garder 100 % sécurisé).",
                s.nom,
                eur(s.deficit_immediat_eur)
            )));
        }
        if let (Some(dans), Some(part)) = (s.prochain_palier_dans_ans, s.prochaine_part_actions) {
            if dans <= 1 {
                alertes.push(Alerte::attention(format!(
                    "{} : passer la sous-poche à {} d'actions d'ici un an (glide path).",
                    s.nom,
                    pct(part)
                )));
            }
        }
    }
    let besoin_projets: f64 = p1.iter().map(|s| s.epargne_mensuelle_eur).fold(0.0, |a, b| a + b);

    // ---------- Poche 2 : profil et cible ----------
    let (sc, detail_score) = score(q, h);
    let profil_score = profil_depuis_score(sc, h);
    let ctx = contexte(q, h);
    let cible = allocation_cible(q, h, &ctx, profil_score);
    let pp = h.profil(cible.profil);

    // ---------- Crédits : coût, cohérence et arbitrage ----------
    // Le taux d'un crédit ne se juge pas dans l'absolu mais face au rendement que la
    // poche 2 peut espérer, net de fiscalité : rembourser, c'est « gagner » le taux du
    // crédit sans risque.
    let rendement_net = pp.rendement * (1.0 - h.pfu);
    let (credits, dettes_cheres, alertes_credits) = analyser_credits(q, h, rendement_net);
    alertes.extend(alertes_credits);

    // ---------- Ventes immobilières programmées ----------
    let ventes: Vec<cession::Cession> = q.ventes.iter().map(|v| cession::cession(q, v, h)).collect();
    for v in &ventes {
        if v.produit_net_eur <= 0.0 {
            alertes.push(Alerte::attention(format!(
                "{} : la vente ne dégage aucun produit net une fois le crédit soldé et l'impôt payé.",
                v.libelle
            )));
        }
        if v.cash_flow_perdu_mensuel_eur > 0.0 {
            alertes.push(Alerte::info(format!(
                "{} : la vente fait perdre {} de revenu passif par mois, ce qui relève d'autant le capital à constituer.",
                v.libelle,
                eur(v.cash_flow_perdu_mensuel_eur)
            )));
        }
        if v.amortissements_reintegres_eur > 0.0 {
            alertes.push(Alerte::attention(format!(
                "{} : {} d'amortissements déduits par la SCI sont réintégrés dans la plus-value, d'où un impôt de cession de {}.",
                v.libelle,
                eur(v.amortissements_reintegres_eur),
                eur(v.impot_eur)
            )));
        }
    }

    // Le capital à constituer se juge sur le revenu passif qui restera à l'échéance :
    // un bien vendu d'ici là ne rapporte plus de loyers.
    let cf_immo_aujourdhui = q.cf_immo_total(h);
    let cf_immo_a_terme = (cf_immo_aujourdhui
        - ventes
            .iter()
            .filter(|v| v.dans_ans <= q.h_fire)
            .map(|v| v.cash_flow_perdu_mensuel_eur)
            .fold(0.0, |a, b| a + b))
    .max(0.0);

    let r_manquant = (q.r_cible - cf_immo_a_terme - q.autres_revenus_passifs).max(0.0);
    let taux_retrait = q.taux_retrait_pct / 100.0;
    let capital_cible = if taux_retrait > 0.0 { r_manquant * 12.0 / taux_retrait } else { 0.0 };
    if taux_retrait >= 0.04 {
        alertes.push(Alerte::attention(format!(
            "Taux de retrait de {} : hypothèse agressive (prudent : 3–3,5 %).",
            pct(taux_retrait)
        )));
    }

    let actuel_eur = allocation_actuelle_eur(q);
    let liquidites = q.avoirs.liquidites_a_investir + p0.excedent_eur;
    // Capital financier : tout sauf l'immobilier locatif (ses revenus sont déjà comptés
    // dans le cash-flow immobilier).
    let capital_fin = actuel_eur.total() - q.v_immo_loc + liquidites;
    let rendement = pp.rendement;

    let besoin_p2 = if q.h_fire == 0 { 0.0 } else { pmt(capital_cible, capital_fin, rendement, q.h_fire) };
    let flux_regime = (f.e_mois - besoin_projets).max(0.0);
    // `fold(0.0, …)` plutôt que `sum()` : la somme d'un itérateur vide de f64 vaut -0,0.
    let du_cher = dettes_cheres;

    // Épargne libérée au fil du temps : fin des crédits du foyer, fin de l'effort sur une
    // SCI vendue, et mensualités que la vente d'un bien fait disparaître.
    let vente_directe = q.ventes.iter().find(|v| matches!(&v.bien, BienVendu::LocatifDirect));
    let mut liberations: Vec<(u32, f64)> = q
        .dettes_foyer()
        .filter(|d| vente_directe.is_none() || d.objet != ObjetCredit::Locatif)
        .filter_map(|d| d.fin_mois().map(|fin| (fin, d.mensualite)))
        .collect();
    let bilans = q.bilans_scis(h);
    for (v, c) in q.ventes.iter().zip(ventes.iter()) {
        match &v.bien {
            BienVendu::LocatifDirect => {
                liberations.push((v.dans_ans * 12, c.mensualites_liberees_eur));
                for d in q.dettes_foyer().filter(|d| d.objet == ObjetCredit::Locatif) {
                    if let Some(fin) = d.fin_mois() {
                        liberations.push((fin, d.mensualite * (1.0 - v.part_vendue())));
                    }
                }
            }
            BienVendu::Sci { id } => {
            if let Some(b) = bilans.iter().find(|b| &b.id == id) {
                liberations.push((v.dans_ans * 12, b.effort_mensuel_eur * v.part_vendue()));
                let total_mensualites = q
                    .dettes
                    .iter()
                    .filter(|d| d.sci_id.as_deref() == Some(id))
                    .map(|d| d.mensualite)
                    .sum::<f64>();
                for d in q.dettes.iter().filter(|d| d.sci_id.as_deref() == Some(id)) {
                    if let Some(fin) = d.fin_mois() {
                        if total_mensualites > 0.0 {
                            liberations.push((
                                fin,
                                b.effort_mensuel_eur * d.mensualite / total_mensualites * (1.0 - v.part_vendue()),
                            ));
                        }
                    }
                }
            }
        }
        }
    }
    let apports: Vec<(u32, f64)> =
        ventes.iter().map(|v| (v.dans_ans * 12, v.produit_net_eur)).collect();

    let sim = Simulation {
        capital: capital_fin,
        e_mois: f.e_mois,
        a_combler: p0.manque_eur + du_cher,
        poches: &p1,
        rendement,
        liberations,
        apports,
    };
    let mois_fire = q.h_fire * 12;
    let capital_simule = sim.capital_a(mois_fire);
    let flux_moyen = if mois_fire > 0 { sim.flux_poche2_cumule(mois_fire) / mois_fire as f64 } else { flux_regime };
    let atteignable = capital_simule + 1.0 >= capital_cible;

    // ---------- Flux mensuels (section 5) ----------
    let mut etapes = Vec::new();
    let mut reste = f.e_mois;
    let precaution_flux = reste.min(p0.manque_eur);
    reste -= precaution_flux;
    if precaution_flux > 0.0 {
        etapes.push(format!("1. {} vers l'épargne de précaution.", eur(precaution_flux)));
    } else {
        etapes.push("1. Épargne de précaution constituée : rien à verser.".into());
    }
    let dettes_flux = reste.min(du_cher);
    reste -= dettes_flux;
    if dettes_flux > 0.0 {
        etapes.push(format!("2. {} en remboursement anticipé des crédits > {}.", eur(dettes_flux), pct(h.seuil_taux_dette)));
    } else {
        etapes.push(format!("2. Aucun crédit à plus de {} à rembourser.", pct(h.seuil_taux_dette)));
    }
    let montants = poche1::repartir_par_priorite(reste, &p1);
    let flux_projets: Vec<FluxProjet> = p1
        .iter()
        .zip(&montants)
        .map(|(s, m)| FluxProjet {
            nom: s.nom.clone(),
            priorite: s.priorite,
            besoin_eur: s.epargne_mensuelle_eur,
            montant_eur: *m,
        })
        .collect();
    let total_projets: f64 = montants.iter().fold(0.0, |a, b| a + b);
    reste -= total_projets;
    if !p1.is_empty() {
        etapes.push(format!(
            "3. {} vers les études et projets, par ordre de priorité.",
            eur(total_projets)
        ));
        let sous_finances: Vec<String> = flux_projets
            .iter()
            .filter(|x| x.besoin_eur > 0.5 && x.montant_eur + 0.5 < x.besoin_eur)
            .map(|x| format!("{} ({} / {})", x.nom, eur(x.montant_eur), eur(x.besoin_eur)))
            .collect();
        if !sous_finances.is_empty() {
            alertes.push(Alerte::critique(format!(
                "Flux insuffisant pour : {}. Les projets essentiels passent en premier ; reporter ou réduire les projets souhaitables.",
                sous_finances.join(", ")
            )));
        }
    }
    let flux_p2 = reste.max(0.0);
    etapes.push(format!("4. {} vers la poche 2, sur les lignes sous-pondérées.", eur(flux_p2)));

    // ---------- Répartition par ligne ----------
    let deploiement = repartir(liquidites, &actuel_eur, &cible.allocation, &cible.gelees);
    let apres_deploiement = actuel_eur.map(|l, v| v + deploiement.get(l));
    let flux_lignes = repartir(flux_p2, &apres_deploiement, &cible.allocation, &cible.gelees);
    let base_totale = actuel_eur.total() + liquidites;

    let mut reequilibrage = false;
    let lignes: Vec<LigneAllocation> = Ligne::TOUTES
        .iter()
        .filter(|l| cible.allocation.get(**l) > 0.0 || actuel_eur.get(**l) > 0.0)
        .map(|l| {
            let c = cible.allocation.get(*l);
            let a = ctx.actuelle_pct.get(*l);
            let ecart = a - c;
            if ctx.base_eur > 0.0 && ecart.abs() > h.tolerance_reequilibrage {
                reequilibrage = true;
            }
            let (env, sup) = enveloppe_et_support(*l, q, h, cible.immo_fige);
            LigneAllocation {
                ligne: *l,
                libelle: l.libelle().into(),
                cible_pct: c,
                actuelle_pct: a,
                ecart_points: ecart * 100.0,
                actuel_eur: actuel_eur.get(*l),
                cible_eur: c * base_totale,
                deploiement_eur: deploiement.get(*l),
                flux_mensuel_eur: flux_lignes.get(*l),
                gelee: cible.gelees.contains(l),
                enveloppe: env,
                support: sup,
            }
        })
        .collect();

    // ---------- Contrôles de cohérence (3.4) ----------
    if ctx.base_eur > 0.0 {
        let act = ctx.actuelle_pct;
        if act.crowdfunding > h.crowdfunding_max {
            alertes.push(Alerte::attention(format!(
                "Crowdfunding à {} > {} : ne plus réinvestir les remboursements, laisser courir les projets, diversifier vers l'ENR.",
                pct(act.crowdfunding),
                pct(h.crowdfunding_max)
            )));
        }
        if act.actions_directes > h.actions_directes_max {
            alertes.push(Alerte::attention(format!(
                "Actions en direct à {} > {}.",
                pct(act.actions_directes),
                pct(h.actions_directes_max)
            )));
        }
        if act.satellites() > h.satellites_max {
            alertes.push(Alerte::attention(format!(
                "Satellites cumulés à {} > {} (actions directes + crowdfunding + crypto + PE).",
                pct(act.satellites()),
                pct(h.satellites_max)
            )));
        }
        if act.or > h.or_max {
            alertes.push(Alerte::attention(format!("Or à {} > {} : ne génère aucun flux.", pct(act.or), pct(h.or_max))));
        }
        if ctx.immo_total_pct > cible.plafond_immo {
            alertes.push(Alerte::attention(format!(
                "Immobilier total (locatif + SCPI + crowdfunding immo) à {} > plafond {} : pas de nouveau flux immobilier.",
                pct(ctx.immo_total_pct),
                pct(cible.plafond_immo)
            )));
        }
        if act.fonds_euros_obligations < h.plancher_fonds_euros {
            alertes.push(Alerte::info(format!(
                "Fonds euros / obligations à {} < {} : réserve de rééquilibrage à reconstituer.",
                pct(act.fonds_euros_obligations),
                pct(h.plancher_fonds_euros)
            )));
        }
        if act.actions() > cible.plafond_actions + 0.02 {
            alertes.push(Alerte::attention(format!(
                "Part actions actuelle {} > plafond {}.",
                pct(act.actions()),
                pct(cible.plafond_actions)
            )));
        }
        if reequilibrage {
            alertes.push(Alerte::info(format!(
                "Au moins une ligne s'écarte de plus de ±{} points de sa cible : rééquilibrer (arbitrer d'abord dans l'AV, les lignes illiquides seulement à l'achat).",
                (h.tolerance_reequilibrage * 100.0).round()
            )));
        }
    }
    if cible.immo_fige {
        alertes.push(Alerte::attention(format!(
            "Immobilier locatif = {} du patrimoine (> {}) : question ouverte — conserver, vendre ou arbitrer.",
            pct(synthese.ratio_immo_locatif),
            pct(h.seuil_concentration_immo)
        )));
    }
    if q.avoirs.etf_monde + q.avoirs.actions_directes > h.plafond_pea {
        alertes.push(Alerte::info(format!(
            "Actions > {} : le PEA est probablement plein, poursuivre en assurance-vie.",
            eur(h.plafond_pea)
        )));
    }
    if q.contraintes.refus_crypto && q.avoirs.crypto > 0.0 {
        alertes.push(Alerte::info("Crypto détenue alors que vous la refusez : envisager de solder la ligne.".to_string()));
    }
    if q.avoirs.private_equity > 0.0 {
        alertes.push(Alerte::info("Private equity : compté dans les satellites, pas de nouvel apport prévu.".to_string()));
    }
    if f.abondement_employeur {
        alertes.push(Alerte::info("Abondement PEE/PER employeur : à capter en priorité, avant tout autre placement.".to_string()));
    }
    if q.tmi.haute() {
        alertes.push(Alerte::info(format!(
            "TMI {} % : activer un PER pour la part retraite (déduction jusqu'à {} des revenus) ; SCPI et obligations en enveloppe capitalisante.",
            q.tmi.pct(),
            pct(h.plafond_per_revenus)
        )));
    } else if q.tmi.basse() {
        alertes.push(Alerte::info("TMI ≤ 11 % : PER peu utile ; SCPI en direct envisageable.".to_string()));
    }
    if q.h_fire > 0 && q.h_fire <= h.annees_derisquage {
        alertes.push(Alerte::attention(format!(
            "Échéance dans {} an(s) : passer la poche 2 au profil {} (risque de séquence) et constituer {} ans de retraits en fonds euros.",
            q.h_fire,
            cible.profil.inferieur().libelle(),
            h.annees_coussin
        )));
    }

    // ---------- Leviers ----------
    let mut leviers = Vec::new();
    if !atteignable {
        alertes.push(Alerte::critique(format!(
            "Objectif revenus passifs non atteignable : capital projeté de {} dans {} ans pour une cible de {} ({} / mois en moyenne vers la poche 2, une fois les projets financés).",
            eur(capital_simule),
            q.h_fire,
            eur(capital_cible),
            eur(flux_moyen)
        )));
        match (0..=60u32).find(|a| sim.capital_a(a * 12) + 1.0 >= capital_cible) {
            Some(a) => leviers.push(Levier {
                titre: "1. Repousser l'échéance".into(),
                detail: format!("Avec le flux actuel, l'objectif est atteint dans {a} ans (au lieu de {}).", q.h_fire),
            }),
            None => leviers.push(Levier {
                titre: "1. Repousser l'échéance".into(),
                detail: "Même en 60 ans, le flux actuel ne suffit pas : combiner avec les autres leviers.".into(),
            }),
        }
        let r_possible = capital_simule * taux_retrait / 12.0 + q.cf_immo + q.autres_revenus_passifs;
        leviers.push(Levier {
            titre: "2. Baisser le revenu visé".into(),
            detail: format!(
                "À {} ans, le revenu passif soutenable serait d'environ {} / mois (temps partiel plutôt que sortie complète).",
                q.h_fire,
                eur(r_possible)
            ),
        });
        leviers.push(Levier {
            titre: "3. Augmenter l'épargne".into(),
            detail: if q.h_fire > 0 {
                format!(
                    "Il faudrait {} de plus par mois vers la poche 2.",
                    eur(pmt(capital_cible - capital_simule, 0.0, rendement, q.h_fire))
                )
            } else {
                format!("Il manque {} de capital.", eur(capital_cible - capital_simule))
            },
        });
        let souhaitables: f64 = p1
            .iter()
            .filter(|s| s.priorite == Priorite::Souhaitable)
            .map(|s| s.epargne_mensuelle_eur)
            .fold(0.0, |a, b| a + b);
        if souhaitables > 0.5 {
            leviers.push(Levier {
                titre: "4. Revoir les projets souhaitables".into(),
                detail: format!(
                    "Les reporter ou les réduire libérerait jusqu'à {} par mois pendant leur durée.",
                    eur(souhaitables)
                ),
            });
        }
        if f.temps_gestion >= TempsGestion::Moyen
            && synthese.taux_endettement < h.endettement_max
            && !q.contraintes.refus_immo_direct
            && !cible.immo_fige
        {
            leviers.push(Levier {
                titre: format!("{}. Effet de levier immobilier", leviers.len() + 1),
                detail: format!(
                    "Endettement actuel {} < {} et temps disponible : un investissement locatif financé à crédit peut accélérer (à étudier au cas par cas).",
                    pct(synthese.taux_endettement),
                    pct(h.endettement_max)
                ),
            });
        }
    }

    // ---------- Projection ----------
    let n = q.h_fire.max(1);
    let projection = (0..=n)
        .map(|a| PointProjection {
            annee: a,
            capital_flux_disponible: sim.capital_a(a * 12),
            capital_epargne_necessaire: fv(capital_fin, besoin_p2, rendement, a * 12),
            cible: capital_cible,
        })
        .collect();

    let mut ajustements = cible.ajustements.clone();
    if cible.profil != profil_score && ajustements.is_empty() {
        ajustements.push(format!("Profil {} retenu.", cible.profil.libelle()));
    }

    let poche_2 = Poche2 {
        profil: cible.profil,
        profil_libelle: cible.profil.libelle().into(),
        score: sc,
        detail_score,
        rendement_attendu: rendement,
        perte_max_plausible: pp.perte_max,
        revenu_manquant_mensuel: r_manquant,
        capital_cible_eur: capital_cible,
        capital_financier_actuel_eur: capital_fin,
        base_allocation_eur: ctx.base_eur,
        epargne_mensuelle_necessaire_eur: besoin_p2,
        flux_disponible_regime_eur: flux_regime,
        flux_moyen_simule_eur: flux_moyen,
        capital_projete_eur: capital_simule,
        atteignable,
        allocation_cible: cible.allocation,
        allocation_actuelle: ctx.actuelle_pct,
        lignes,
        ajustements,
        leviers,
        projection,
        reequilibrage_necessaire: reequilibrage,
    };

    // ---------- Phase revenus passifs (section 6) ----------
    let ps = pct(h.prelevements_sociaux);
    let phase_revenus = PhaseRevenus {
        annee_derisquage: q.h_fire.saturating_sub(h.annees_derisquage),
        profil_phase: cible.profil.inferieur(),
        retrait_annuel_eur: capital_cible * taux_retrait,
        retrait_mensuel_eur: capital_cible * taux_retrait / 12.0,
        coussin_fonds_euros_eur: r_manquant * 12.0 * h.annees_coussin as f64,
        abattement_av_eur: if f.en_couple { h.abattement_av_couple } else { h.abattement_av_seul },
        ordre_sources: vec![
            format!(
                "Rachats partiels AV > 8 ans (abattement annuel {} sur les gains)",
                eur(if f.en_couple { h.abattement_av_couple } else { h.abattement_av_seul })
            ),
            format!("Retraits PEA > 5 ans (gains soumis aux seuls prélèvements sociaux, {ps})"),
            "Loyers SCPI / locatif".into(),
            format!("Coupons de crowdfunding (PFU {})", pct(h.pfu)),
            "CTO".into(),
        ],
        regles: vec![
            format!("Passer au profil {} {} ans avant l'échéance.", cible.profil.inferieur().libelle(), h.annees_derisquage),
            format!("Retirer {} du capital financier par an, indexé sur l'inflation.", pct(taux_retrait)),
            "Suspendre l'indexation après une année à -15 %.".into(),
            format!("Garder {} ans de retraits en fonds euros (coussin).", h.annees_coussin),
        ],
    };

    let mut priorite_enveloppes = Vec::new();
    if f.abondement_employeur {
        priorite_enveloppes.push("PEE / PER employeur (abondement)".to_string());
    }
    priorite_enveloppes.push("PEA — ETF monde".into());
    priorite_enveloppes.push("Assurance-vie — fonds euros + ETF + SCPI".into());
    if q.tmi.haute() {
        priorite_enveloppes.push("PER individuel (TMI ≥ 30 %, horizon retraite)".into());
    }
    priorite_enveloppes.push("CTO — or, ETF non éligibles".into());
    priorite_enveloppes.push("Plateformes de crowdfunding".into());

    // ---------- Immobilier : SCI, ventes et crédits ----------
    for b in &bilans {
        for c in &b.commentaires {
            alertes.push(Alerte::info(format!("{} : {}", b.nom, c)));
        }
        // Le choix du régime se juge à la TMI : à l'IR, les loyers s'ajoutent aux revenus
        // du foyer et sont imposés au barème plus les prélèvements sociaux.
        if b.regime == RegimeSci::Ir && q.tmi.haute() && b.resultat_imposable_eur > 0.0 {
            alertes.push(Alerte::attention(format!(
                "{} : SCI à l'IR avec une TMI de {} %, soit {} d'impôt et de prélèvements sociaux par an sur un résultat de {}. L'IS permettrait d'amortir le bien, au prix d'une plus-value plus lourde à la revente.",
                b.nom,
                q.tmi.pct(),
                eur(b.impot_annuel_eur),
                eur(b.resultat_imposable_eur)
            )));
        }
    }

    let immobilier = Immobilier {
        locatif_direct_net_eur: q.v_immo_loc,
        cash_flow_direct_mensuel_eur: q.cf_immo,
        total_net_eur: immo_net,
        cash_flow_total_mensuel_eur: cf_immo_aujourdhui,
        effort_scis_mensuel_eur: f.effort_scis,
        capitalise_scis_annuel_eur: bilans
            .iter()
            .map(|b| b.capitalise_annuel_eur)
            .fold(0.0, |a, b| a + b),
        scis: bilans,
        ventes,
        credits,
    };

    // Tri des alertes : critique > attention > info.
    alertes.sort_by_key(|a| match a.gravite {
        Gravite::Critique => 0,
        Gravite::Attention => 1,
        Gravite::Info => 2,
    });

    Resultat {
        synthese,
        budget,
        immobilier,
        poche_0: p0,
        poche_1: p1,
        poche_2,
        flux: RepartitionFlux {
            total_eur: f.e_mois,
            precaution_eur: precaution_flux,
            remboursement_dettes_eur: dettes_flux,
            projets: flux_projets,
            poche_2_eur: flux_p2,
            etapes,
        },
        priorite_enveloppes,
        phase_revenus,
        alertes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    fn somme_flux(r: &Resultat) -> f64 {
        let f = &r.flux;
        f.precaution_eur + f.remboursement_dettes_eur + f.projets.iter().map(|e| e.montant_eur).sum::<f64>() + f.poche_2_eur
    }

    #[test]
    fn exemple_de_la_spec_capital_cible() {
        let mut q = Questionnaire::exemple();
        q.cf_immo = 0.0;
        let r = calculer(&q, &Hypotheses::default());
        // 1 500 × 12 / 0,035 ≈ 514 286 €
        assert!(approx(r.poche_2.capital_cible_eur, 514_285.71, 1.0));
    }

    #[test]
    fn flux_conserves() {
        let q = Questionnaire::exemple();
        let e = q.foyer(&Hypotheses::default()).e_mois;
        let r = calculer(&q, &Hypotheses::default());
        assert!(approx(somme_flux(&r), e, 1e-6));
        assert!(r.flux.remboursement_dettes_eur.is_sign_positive(), "pas de -0");
        let lignes: f64 = r.poche_2.lignes.iter().map(|l| l.flux_mensuel_eur).sum();
        assert!(approx(lignes, r.flux.poche_2_eur, 1e-6));
        let cible: f64 = r.poche_2.lignes.iter().map(|l| l.cible_pct).sum();
        assert!(approx(cible, 1.0, 1e-9));
        // 2 enfants + 2 projets, études en premier.
        assert_eq!(r.poche_1.len(), 4);
        assert_eq!(r.poche_1[0].categorie, CategoriePoche::Etudes);
        assert_eq!(r.poche_1[3].priorite, Priorite::Souhaitable);
    }

    #[test]
    fn budget_du_foyer() {
        let q = Questionnaire::exemple();
        let r = calculer(&q, &Hypotheses::default());
        let b = &r.budget;
        assert_eq!(b.adultes.len(), 2);
        assert!(approx(b.total_revenus_eur, 5_800.0, 1e-9));
        // 5 800 − 2 670 de dépenses − 1 050 de crédit − l'effort versé à la SCI.
        let effort = r.immobilier.effort_scis_mensuel_eur;
        assert!(effort > 0.0);
        assert!(approx(b.capacite_calculee_eur, 2_080.0 - effort, 1e-9));
        assert!(approx(b.epargne_retenue_eur, 2_080.0 - effort, 1e-9));
        assert_eq!(b.postes.len(), 9);
        assert!(approx(b.adultes.iter().map(|a| a.part).sum::<f64>(), 1.0, 1e-9));
        // Précaution sur dépenses + mensualités : 4 mois × 3 720 €.
        assert!(approx(r.poche_0.cible_eur, 4.0 * 3_720.0, 1e-9));
        assert_eq!(r.synthese.nb_adultes, 2);
        assert_eq!(r.synthese.tol_risque_retenue, 3);
    }

    #[test]
    fn budget_deficitaire() {
        let mut q = Questionnaire::exemple();
        q.depenses.push(Depense { poste: PosteDepense::Autre, libelle: "x".into(), montant_mensuel: 3_000.0 });
        let r = calculer(&q, &Hypotheses::default());
        assert_eq!(r.budget.epargne_retenue_eur, 0.0);
        assert!(r.alertes.iter().any(|a| a.message.starts_with("Budget déficitaire")));
        assert!(approx(somme_flux(&r), 0.0, 1e-9));
    }

    #[test]
    fn ecart_de_tolerance_signale() {
        let mut q = Questionnaire::exemple();
        q.adultes[0].tol_risque = 5;
        q.adultes[1].tol_risque = 2;
        let r = calculer(&q, &Hypotheses::default());
        assert!(r.alertes.iter().any(|a| a.message.contains("Rapports au risque")));
        assert_eq!(r.synthese.tol_risque_retenue, 2);
    }

    #[test]
    fn precaution_prioritaire() {
        let mut q = Questionnaire::exemple();
        q.avoirs.livrets = 0.0;
        let e = q.foyer(&Hypotheses::default()).e_mois;
        let r = calculer(&q, &Hypotheses::default());
        assert!(approx(r.flux.precaution_eur, e, 1e-9));
        assert_eq!(r.flux.poche_2_eur, 0.0);
        assert!(r.alertes.iter().any(|a| a.gravite == Gravite::Critique));
    }

    #[test]
    fn dette_chere_absorbe_le_flux() {
        let mut q = Questionnaire::exemple();
        q.dettes.push(Dette {
            libelle: "Conso".into(),
            taux_pct: 7.0,
            restant_du: 500.0,
            mensualite: 100.0,
            duree_restante_mois: 6,
            objet: ObjetCredit::Consommation,
            sci_id: None,
        });
        let r = calculer(&q, &Hypotheses::default());
        assert!(approx(r.flux.remboursement_dettes_eur, 500.0, 1e-9));
    }

    #[test]
    fn projets_souhaitables_servis_en_dernier() {
        let mut q = Questionnaire::exemple();
        q.epargne_mensuelle_forcee = Some(1_000.0);
        let r = calculer(&q, &Hypotheses::default());
        let voyage = r.flux.projets.iter().find(|p| p.nom == "Voyage en famille").unwrap();
        assert_eq!(voyage.montant_eur, 0.0);
        let etudes: f64 = r.flux.projets.iter().filter(|p| p.nom.starts_with("Études")).map(|p| p.montant_eur).sum();
        let besoin: f64 = r.flux.projets.iter().filter(|p| p.nom.starts_with("Études")).map(|p| p.besoin_eur).sum();
        assert!(approx(etudes, besoin, 1e-6));
        assert!(r.alertes.iter().any(|a| a.message.starts_with("Flux insuffisant")));
    }

    #[test]
    fn flux_libere_apres_les_projets() {
        let q = Questionnaire::exemple();
        let r = calculer(&q, &Hypotheses::default());
        let p2 = &r.poche_2;
        // Les projets à 2 et 4 ans libèrent du flux : la moyenne dépasse le flux actuel.
        assert!(p2.flux_moyen_simule_eur > p2.flux_disponible_regime_eur);
        let dernier = p2.projection.last().unwrap();
        assert!(approx(dernier.capital_flux_disponible, p2.capital_projete_eur, 1e-6));
    }

    #[test]
    fn crowdfunding_excessif_gele() {
        let q = Questionnaire::exemple(); // 9 k€ de crowdfunding sur ~108 k€
        let r = calculer(&q, &Hypotheses::default());
        let cf = r.poche_2.lignes.iter().find(|l| l.ligne == Ligne::Crowdfunding).unwrap();
        assert!(cf.gelee);
        assert_eq!(cf.flux_mensuel_eur, 0.0);
        assert!(r.alertes.iter().any(|a| a.message.contains("ne plus réinvestir")));
    }

    #[test]
    fn non_atteignable_propose_leviers() {
        let mut q = Questionnaire::exemple();
        q.r_cible = 10_000.0;
        let r = calculer(&q, &Hypotheses::default());
        assert!(!r.poche_2.atteignable);
        assert!(r.poche_2.leviers.len() >= 4);
    }

    #[test]
    fn serialisation_json() {
        let q = Questionnaire::exemple();
        let s = serde_json::to_string(&q).unwrap();
        assert!(s.contains("\"tmi\":\"30\""));
        assert!(s.contains("\"epargne_mensuelle_forcee\":null"));
        let q2: Questionnaire = serde_json::from_str(&s).unwrap();
        assert_eq!(q, q2);
        let r = calculer(&q, &Hypotheses::default());
        let js = serde_json::to_value(&r).unwrap();
        assert!(js["poche_2"]["allocation_cible"]["etf_monde"].is_number());
        assert!(js["budget"]["postes"].is_array());
    }

    #[test]
    fn sans_patrimoine_ni_enfant() {
        let mut q = Questionnaire::exemple();
        q.enfants.clear();
        q.projets.clear();
        q.dettes.clear();
        q.depenses.clear();
        q.avoirs = Avoirs::default();
        q.v_immo_loc = 0.0;
        q.scis.clear();
        q.ventes.clear();
        let e = q.foyer(&Hypotheses::default()).e_mois;
        let r = calculer(&q, &Hypotheses::default());
        let somme: f64 = r.poche_2.lignes.iter().map(|l| l.flux_mensuel_eur).sum();
        assert!(approx(somme, e, 1e-6));
        let etf = r.poche_2.lignes.iter().find(|l| l.ligne == Ligne::EtfMonde).unwrap();
        assert!(approx(etf.flux_mensuel_eur, e * 0.45, 1e-6));
    }

    /// Un questionnaire sans SCI ni vente doit se comporter exactement comme avant.
    fn q_simple() -> Questionnaire {
        let mut q = Questionnaire::exemple();
        q.scis.clear();
        q.ventes.clear();
        q.dettes.retain(|d| d.sci_id.is_none());
        q
    }

    #[test]
    fn la_fin_d_un_credit_libere_l_epargne() {
        let h = Hypotheses::default();
        let mut q = q_simple();
        q.enfants.clear();
        q.projets.clear();
        q.h_fire = 20;

        // Même crédit, avec puis sans durée renseignée.
        q.dettes = vec![Dette {
            libelle: "Crédit RP".into(),
            taux_pct: 1.3,
            restant_du: 100_000.0,
            mensualite: 1_000.0,
            duree_restante_mois: 0,
            objet: ObjetCredit::ResidencePrincipale,
            sci_id: None,
        }];
        let sans_duree = calculer(&q, &h).poche_2.capital_projete_eur;

        q.dettes[0].duree_restante_mois = 60;
        let r = calculer(&q, &h);
        // La mensualité revient à l'épargne au bout de 5 ans : le capital projeté grimpe.
        assert!(r.poche_2.capital_projete_eur > sans_duree, "{} vs {sans_duree}", r.poche_2.capital_projete_eur);
        assert!(r.poche_2.flux_moyen_simule_eur > r.poche_2.flux_disponible_regime_eur);
        // La capacité d'épargne d'aujourd'hui, elle, ne change pas.
        assert!(approx(r.budget.capacite_calculee_eur, calculer(&q, &h).budget.capacite_calculee_eur, 1e-9));
    }

    #[test]
    fn arbitrage_rembourser_ou_investir() {
        let h = Hypotheses::default();
        let mut q = q_simple();
        let credit = |taux, objet| Dette {
            libelle: "Crédit".into(),
            taux_pct: taux,
            restant_du: 20_000.0,
            mensualite: 300.0,
            duree_restante_mois: 84,
            objet,
            sci_id: None,
        };

        // Taux très élevé : remboursement prioritaire, et le flux y est effectivement affecté.
        q.dettes = vec![credit(9.0, ObjetCredit::Consommation)];
        let r = calculer(&q, &h);
        assert_eq!(r.immobilier.credits[0].verdict, VerdictCredit::Rembourser);
        assert!(r.flux.remboursement_dettes_eur > 0.0);

        // Taux intermédiaire : arbitrage signalé, mais le flux n'est pas préempté.
        q.dettes = vec![credit(4.5, ObjetCredit::Consommation)];
        let r = calculer(&q, &h);
        assert_eq!(r.immobilier.credits[0].verdict, VerdictCredit::Arbitrer);
        assert_eq!(r.flux.remboursement_dettes_eur, 0.0);

        // Taux bas sur un bien qui rapporte : c'est du levier, on conserve.
        q.dettes = vec![credit(1.5, ObjetCredit::Locatif)];
        let r = calculer(&q, &h);
        assert_eq!(r.immobilier.credits[0].verdict, VerdictCredit::Levier);

        // Même taux bas, mais sans bien productif derrière.
        q.dettes = vec![credit(1.5, ObjetCredit::ResidencePrincipale)];
        let r = calculer(&q, &h);
        assert_eq!(r.immobilier.credits[0].verdict, VerdictCredit::Conserver);
    }

    #[test]
    fn mensualite_incoherente_signalee() {
        let h = Hypotheses::default();
        let mut q = q_simple();
        q.dettes = vec![Dette {
            libelle: "Crédit".into(),
            taux_pct: 3.0,
            restant_du: 100_000.0,
            mensualite: 300.0, // très loin des ~1 380 € attendus sur 84 mois
            duree_restante_mois: 84,
            objet: ObjetCredit::Autre,
            sci_id: None,
        }];
        let r = calculer(&q, &h);
        let c = &r.immobilier.credits[0];
        assert!(!c.coherent);
        assert!(c.mensualite_theorique_eur > 1_300.0);
        assert!(r.alertes.iter().any(|a| a.message.contains("la mensualité devrait être")));

        // Mensualité cohérente : plus d'alerte, et des intérêts positifs.
        q.dettes[0].mensualite = c.mensualite_theorique_eur;
        let r = calculer(&q, &h);
        assert!(r.immobilier.credits[0].coherent);
        assert!(r.immobilier.credits[0].interets_restants_eur > 0.0);
    }

    #[test]
    fn credit_de_sci_hors_capacite_mais_dans_la_vue_bancaire() {
        let h = Hypotheses::default();
        let q = Questionnaire::exemple();
        let r = calculer(&q, &h);
        // La mensualité de la SCI ne pèse pas sur la capacité d'épargne du foyer…
        assert!(approx(r.budget.mensualites_credits_eur, 1_050.0, 1e-9));
        // … mais elle compte dans la vue bancaire, plus élevée que le taux du foyer seul.
        assert!(r.synthese.taux_endettement_bancaire > r.synthese.taux_endettement);
        assert_eq!(r.immobilier.credits.len(), 2);
        assert!(r.immobilier.credits.iter().any(|c| c.porte_par == "SCI du Moulin"));
    }

    #[test]
    fn sci_a_l_is_ne_produit_pas_de_revenu_passif_sans_distribution() {
        let h = Hypotheses::default();
        let mut q = q_simple();
        q.cf_immo = 0.0;
        q.v_immo_loc = 0.0;
        q.scis = vec![Sci {
            id: "s".into(),
            nom: "SCI".into(),
            regime: RegimeSci::Is,
            part_foyer_pct: 100.0,
            valeur_biens: 200_000.0,
            base_amortissement: 200_000.0,
            scpi: 0.0,
            loyers_mensuels: 1_400.0,
            charges_mensuelles: 200.0,
            distribution_pct: 0.0,
        }];
        let sans = calculer(&q, &h);
        assert_eq!(sans.immobilier.cash_flow_total_mensuel_eur, 0.0);
        assert!(sans.immobilier.capitalise_scis_annuel_eur > 0.0);

        // En distribuant, le foyer touche enfin un revenu passif : le capital à constituer baisse.
        q.scis[0].distribution_pct = 100.0;
        let avec = calculer(&q, &h);
        assert!(avec.immobilier.cash_flow_total_mensuel_eur > 0.0);
        assert!(avec.poche_2.capital_cible_eur < sans.poche_2.capital_cible_eur);
    }

    #[test]
    fn vente_verse_son_produit_et_supprime_les_loyers() {
        let h = Hypotheses::default();
        let mut q = q_simple();
        q.enfants.clear();
        q.projets.clear();
        q.h_fire = 10;
        q.cf_immo = 400.0;
        q.v_immo_loc = 150_000.0;

        let sans_vente = calculer(&q, &h);
        q.ventes = vec![VenteImmobiliere {
            libelle: "Vente de l'appartement".into(),
            bien: BienVendu::LocatifDirect,
            dans_ans: 3,
            part_vendue_pct: 100.0,
            prix_vente: 150_000.0,
            prix_acquisition: 150_000.0,
            detention_ans: 25,
            frais_vente_pct: 0.0,
        }];
        let avec_vente = calculer(&q, &h);

        let c = &avec_vente.immobilier.ventes[0];
        assert!(c.produit_net_eur > 140_000.0);
        // Vendu 3 ans plus tard, le bien s'est revalorisé : il reste une petite plus-value.
        // À 28 ans de détention, l'impôt est éteint (22 ans) mais pas encore les
        // prélèvements sociaux (30 ans), d'où un reliquat modeste.
        assert!(c.impot_eur > 0.0 && c.impot_eur < 500.0, "{}", c.impot_eur);
        assert!((c.cash_flow_perdu_mensuel_eur - 400.0).abs() < 1e-9);
        // Le produit entre dans la poche 2 : le capital projeté augmente nettement…
        assert!(avec_vente.poche_2.capital_projete_eur > sans_vente.poche_2.capital_projete_eur + 100_000.0);
        // … mais les loyers perdus relèvent le capital à constituer.
        assert!(avec_vente.poche_2.capital_cible_eur > sans_vente.poche_2.capital_cible_eur);
    }

    #[test]
    fn vendre_une_sci_a_l_is_coute_les_amortissements() {
        let h = Hypotheses::default();
        let mut q = q_simple();
        let sci = |regime| Sci {
            id: "s".into(),
            nom: "SCI".into(),
            regime,
            part_foyer_pct: 100.0,
            valeur_biens: 300_000.0,
            base_amortissement: 300_000.0,
            scpi: 0.0,
            loyers_mensuels: 1_200.0,
            charges_mensuelles: 200.0,
            distribution_pct: 0.0,
        };
        q.ventes = vec![VenteImmobiliere {
            libelle: "Vente SCI".into(),
            bien: BienVendu::Sci { id: "s".into() },
            dans_ans: 0,
            part_vendue_pct: 100.0,
            prix_vente: 300_000.0,
            prix_acquisition: 300_000.0,
            detention_ans: 12,
            frais_vente_pct: 0.0,
        }];

        // Vendue au prix d'achat : rien à payer pour une SCI à l'IR…
        q.scis = vec![sci(RegimeSci::Ir)];
        let ir = calculer(&q, &h);
        assert_eq!(ir.immobilier.ventes[0].impot_eur, 0.0);

        // … alors qu'à l'IS les 12 ans d'amortissements sont réintégrés et imposés.
        q.scis = vec![sci(RegimeSci::Is)];
        let is = calculer(&q, &h);
        let v = &is.immobilier.ventes[0];
        assert!((v.amortissements_reintegres_eur - 102_000.0).abs() < 1.0);
        assert!(v.impot_eur > 15_000.0);
        assert!(v.produit_net_eur < ir.immobilier.ventes[0].produit_net_eur);
        assert!(is.alertes.iter().any(|a| a.message.contains("réintégrés")));
    }
}
