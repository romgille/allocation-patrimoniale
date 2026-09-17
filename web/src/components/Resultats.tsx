import type { Alerte } from '../bindings/Alerte'
import type { Budget } from '../bindings/Budget'
import type { Priorite } from '../bindings/Priorite'
import type { Resultat } from '../bindings/Resultat'
import { eur, num, pct, pts } from '../format'
import { AllocationChart, FluxBar, ProjectionChart, type SegmentFlux } from './charts'

const ICONES: Record<Alerte['gravite'], string> = { critique: '⛔', attention: '⚠️', info: 'ℹ️' }
const LIBELLES: Record<Alerte['gravite'], string> = { critique: 'Critique', attention: 'Attention', info: 'Info' }
const PRIORITE: Record<Priorite, string> = { essentiel: 'Essentiel', important: 'Important', souhaitable: 'Souhaitable' }
const TEMPS = { faible: 'faible', moyen: 'moyen', eleve: 'élevé' } as const

function BudgetFoyer({ b }: { b: Budget }) {
  const deficit = Math.max(0, -b.capacite_calculee_eur)
  const segments: SegmentFlux[] = [
    { cle: 'depenses', libelle: 'Dépenses courantes', valeur: b.total_depenses_eur, serie: 4 },
    { cle: 'credits', libelle: 'Mensualités de crédit', valeur: b.mensualites_credits_eur, serie: 3 },
    { cle: 'epargne', libelle: 'Épargne retenue', valeur: b.epargne_retenue_eur, serie: 1 },
    { cle: 'reste', libelle: 'Marge non épargnée', valeur: b.non_affecte_eur, serie: 'neutre' },
  ]
  return (
    <section className="bloc">
      <h2>Budget du foyer — {eur(b.total_revenus_eur)} de revenus par mois</h2>
      <FluxBar segments={segments} total={Math.max(b.total_revenus_eur, b.total_depenses_eur + b.mensualites_credits_eur)} />
      {deficit > 0 && <p className="ko">Déficit mensuel : {eur(deficit)}</p>}
      <div className="deux-colonnes">
        <div>
          <h3>Membres du foyer</h3>
          <div className="defilement">
            <table className="tableau compact">
              <thead>
                <tr>
                  <th scope="col">Adulte</th>
                  <th scope="col" className="num">Revenus</th>
                  <th scope="col" className="num">Part</th>
                  <th scope="col" className="num">Risque</th>
                  <th scope="col">Stabilité · temps</th>
                </tr>
              </thead>
              <tbody>
                {b.adultes.map((a) => (
                  <tr key={a.prenom}>
                    <th scope="row">{a.prenom}</th>
                    <td className="num">{eur(a.montant_eur)}</td>
                    <td className="num">{pct(a.part, 0)}</td>
                    <td className="num">{a.tol_risque}/5</td>
                    <td>
                      {a.stab_revenus}/3 · {TEMPS[a.temps_gestion]}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
        <div>
          <h3>Dépenses par poste</h3>
          <div className="defilement">
            <table className="tableau compact">
              <tbody>
                {b.postes.map((p) => (
                  <tr key={p.poste}>
                    <th scope="row">{p.libelle}</th>
                    <td className="num">{eur(p.montant_eur)}</td>
                    <td className="num muet">{pct(p.part_revenus, 0)}</td>
                  </tr>
                ))}
                <tr className="total">
                  <th scope="row">Total dépenses courantes</th>
                  <td className="num">{eur(b.total_depenses_eur)}</td>
                  <td className="num muet">{pct(b.total_revenus_eur > 0 ? b.total_depenses_eur / b.total_revenus_eur : 0, 0)}</td>
                </tr>
              </tbody>
            </table>
          </div>
          <p className="aide">
            Taux d'épargne : <strong>{pct(b.taux_epargne)}</strong>
            {b.epargne_forcee ? ' (montant imposé)' : ' (capacité calculée)'}.
          </p>
        </div>
      </div>
    </section>
  )
}

function Alertes({ alertes }: { alertes: Alerte[] }) {
  if (alertes.length === 0) return <p className="vide">Aucune alerte.</p>
  return (
    <ul className="alertes">
      {alertes.map((a, i) => (
        <li key={i} className={`alerte ${a.gravite}`}>
          <span className="alerte-icone" aria-hidden="true">
            {ICONES[a.gravite]}
          </span>
          <span className="alerte-gravite">{LIBELLES[a.gravite]}</span>
          <span>{a.message}</span>
        </li>
      ))}
    </ul>
  )
}

function Tuile({ titre, valeur, detail, ton }: { titre: string; valeur: string; detail?: string; ton?: 'ok' | 'ko' }) {
  return (
    <div className="tuile">
      <span className="tuile-titre">{titre}</span>
      <span className="tuile-valeur">{valeur}</span>
      {detail && <span className={`tuile-detail ${ton ?? ''}`}>{detail}</span>}
    </div>
  )
}

export function Resultats({ r }: { r: Resultat }) {
  const p2 = r.poche_2
  const segments: SegmentFlux[] = [
    { cle: 'precaution', libelle: 'Épargne de précaution', valeur: r.flux.precaution_eur, serie: 3 },
    { cle: 'dettes', libelle: 'Remboursement de crédits', valeur: r.flux.remboursement_dettes_eur, serie: 4 },
    { cle: 'projets', libelle: 'Études et projets', valeur: r.flux.projets.reduce((s, e) => s + e.montant_eur, 0), serie: 2 },
    { cle: 'poche2', libelle: 'Revenus passifs (poche 2)', valeur: r.flux.poche_2_eur, serie: 1 },
  ]
  const lignesDeploiement = p2.lignes.filter((l) => l.deploiement_eur > 0.5)

  return (
    <div className="resultats">
      <div className="tuiles">
        <Tuile
          titre="Profil poche 2"
          valeur={p2.profil_libelle}
          detail={`Score ${num(p2.score)} · rendement ≈ ${pct(p2.rendement_attendu)} · perte possible ${pct(p2.perte_max_plausible, 0)}`}
        />
        <Tuile
          titre="Capital cible revenus passifs"
          valeur={eur(p2.capital_cible_eur)}
          detail={`pour ${eur(p2.revenu_manquant_mensuel)}/mois manquants`}
        />
        <Tuile
          titre="Épargne nécessaire (poche 2)"
          valeur={`${eur(p2.epargne_mensuelle_necessaire_eur)}/mois`}
          detail={`${p2.atteignable ? '✓ Atteignable' : '✗ Non atteignable'} — ${eur(p2.flux_disponible_regime_eur)}/mois aujourd'hui, ${eur(p2.flux_moyen_simule_eur)} en moyenne une fois les projets financés`}
          ton={p2.atteignable ? 'ok' : 'ko'}
        />
        <Tuile
          titre="Patrimoine financier (hors RP)"
          valeur={eur(r.synthese.p_fin_eur)}
          detail={`+ ${eur(r.synthese.v_immo_loc_eur)} de locatif net (${pct(r.synthese.ratio_immo_locatif, 0)})`}
        />
      </div>

      <BudgetFoyer b={r.budget} />

      <section className="bloc">
        <h2>Alertes et priorités</h2>
        <Alertes alertes={r.alertes} />
      </section>

      <section className="bloc">
        <h2>Répartition de l'épargne mensuelle — {eur(r.flux.total_eur)}</h2>
        <FluxBar segments={segments} total={r.flux.total_eur} />
        <ol className="etapes">
          {r.flux.etapes.map((e, i) => (
            <li key={i}>{e.replace(/^\d+\.\s*/, '')}</li>
          ))}
        </ol>
      </section>

      <div className="deux-colonnes">
        <section className="bloc">
          <h2>Poche 0 — Précaution</h2>
          <dl className="liste-def">
            <dt>Cible</dt>
            <dd>
              {eur(r.poche_0.cible_eur)} <span className="muet">({num(r.poche_0.cible_mois)} mois)</span>
            </dd>
            <dt>Existant</dt>
            <dd>{eur(r.poche_0.existant_eur)}</dd>
            {r.poche_0.manque_eur > 0 && (
              <>
                <dt>Manque</dt>
                <dd className="ko">{eur(r.poche_0.manque_eur)}</dd>
              </>
            )}
            {r.poche_0.excedent_eur > 0 && (
              <>
                <dt>Excédent redéployé</dt>
                <dd>{eur(r.poche_0.excedent_eur)}</dd>
              </>
            )}
            <dt>Supports</dt>
            <dd>{r.poche_0.supports.join(' · ')}</dd>
          </dl>
        </section>

        <section className="bloc">
          <h2>Priorité des enveloppes</h2>
          <ol className="etapes">
            {r.priorite_enveloppes.map((e) => (
              <li key={e}>{e}</li>
            ))}
          </ol>
        </section>
      </div>

      {r.poche_1.length > 0 && (
        <section className="bloc">
          <h2>Poche 1 — Études et projets datés</h2>
          <div className="defilement">
            <table className="tableau">
              <thead>
                <tr>
                  <th scope="col">Poche</th>
                  <th scope="col">Priorité</th>
                  <th scope="col" className="num">Échéance</th>
                  <th scope="col" className="num">Montant (auj.)</th>
                  <th scope="col" className="num">Capital cible</th>
                  <th scope="col" className="num">Déjà affecté</th>
                  <th scope="col" className="num">Besoin / mois</th>
                  <th scope="col" className="num">Versé / mois</th>
                  <th scope="col" className="num">ETF / fonds €</th>
                  <th scope="col">Prochain palier</th>
                </tr>
              </thead>
              <tbody>
                {r.poche_1.map((s, i) => {
                  const verse = r.flux.projets[i]?.montant_eur ?? 0
                  const manque = s.epargne_mensuelle_eur - verse > 0.5
                  return (
                    <tr key={`${s.nom}-${i}`}>
                      <th scope="row">
                        {s.nom}
                        {s.cout_annuel_actuel_eur !== null && (
                          <span className="muet"> · {eur(s.cout_annuel_actuel_eur)}/an</span>
                        )}
                      </th>
                      <td>
                        <span className={`badge priorite-${s.priorite}`}>{PRIORITE[s.priorite]}</span>
                      </td>
                      <td className="num">{s.horizon_ans === 0 ? 'maintenant' : `${s.horizon_ans} an${s.horizon_ans > 1 ? 's' : ''}`}</td>
                      <td className="num">{eur(s.montant_actuel_eur)}</td>
                      <td className="num">{eur(s.capital_cible_eur)}</td>
                      <td className="num">{eur(s.capital_actuel_eur)}</td>
                      <td className="num">
                        {eur(s.epargne_mensuelle_eur)}
                        {s.deficit_immediat_eur > 0 && <span className="ko"> (manque {eur(s.deficit_immediat_eur)})</span>}
                      </td>
                      <td className={`num ${manque ? 'ko' : ''}`}>
                        <strong>{eur(verse)}</strong>
                      </td>
                      <td className="num">
                        {pct(s.allocation.etf_monde, 0)} / {pct(s.allocation.fonds_euros, 0)}
                      </td>
                      <td>
                        {s.prochain_palier_dans_ans !== null && s.prochaine_part_actions !== null
                          ? `dans ${s.prochain_palier_dans_ans} an${s.prochain_palier_dans_ans > 1 ? 's' : ''} → ${pct(s.prochaine_part_actions, 0)} actions`
                          : '100 % sécurisé'}
                      </td>
                    </tr>
                  )
                })}
              </tbody>
            </table>
          </div>
          <ul className="puces">
            {[...new Set(r.poche_1.map((s) => s.enveloppe))].map((e) => (
              <li key={e}>{e}</li>
            ))}
          </ul>
          <p className="aide">
            Rendement de calcul pondéré par la part actions du glide path (hypothèses modifiables). Le PEA reste
            réservé à la poche 2.
          </p>
        </section>
      )}

      <section className="bloc">
        <h2>Poche 2 — Allocation cible ({p2.profil_libelle})</h2>
        {p2.base_allocation_eur > 0 ? (
          <AllocationChart lignes={p2.lignes} />
        ) : (
          <p className="vide">Aucun avoir en poche 2 : l'allocation s'applique aux premiers versements.</p>
        )}
        <div className="defilement">
          <table className="tableau">
            <thead>
              <tr>
                <th scope="col">Ligne</th>
                <th scope="col" className="num">Cible</th>
                <th scope="col" className="num">Actuelle</th>
                <th scope="col" className="num">Écart</th>
                <th scope="col" className="num">Flux / mois</th>
                <th scope="col">Enveloppe</th>
                <th scope="col">Support</th>
              </tr>
            </thead>
            <tbody>
              {p2.lignes.map((l) => (
                <tr key={l.ligne} className={l.gelee ? 'gelee' : ''}>
                  <th scope="row">
                    {l.libelle}
                    {l.gelee && <span className="badge">apports fermés</span>}
                  </th>
                  <td className="num">{pct(l.cible_pct)}</td>
                  <td className="num">{pct(l.actuelle_pct)}</td>
                  <td className={`num ${Math.abs(l.ecart_points) > 5 ? 'fort' : ''}`}>{pts(l.ecart_points)}</td>
                  <td className="num">
                    <strong>{eur(l.flux_mensuel_eur)}</strong>
                  </td>
                  <td>{l.enveloppe}</td>
                  <td>{l.support}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        {lignesDeploiement.length > 0 && (
          <>
            <h3>Déploiement des liquidités disponibles</h3>
            <ul className="puces">
              {lignesDeploiement.map((l) => (
                <li key={l.ligne}>
                  {l.libelle} : <strong>{eur(l.deploiement_eur)}</strong>
                </li>
              ))}
            </ul>
          </>
        )}
        {p2.ajustements.length > 0 && (
          <>
            <h3>Ajustements appliqués au profil</h3>
            <ul className="puces">
              {p2.ajustements.map((a, i) => (
                <li key={i}>{a}</li>
              ))}
            </ul>
          </>
        )}
        <details>
          <summary>Détail du score ({num(p2.score)})</summary>
          <table className="tableau compact">
            <tbody>
              {p2.detail_score.map((d) => (
                <tr key={d.libelle}>
                  <th scope="row">{d.libelle}</th>
                  <td className="num">{d.points > 0 ? `+${num(d.points)}` : num(d.points)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </details>
      </section>

      <section className="bloc">
        <h2>Trajectoire vers le capital cible</h2>
        <p className="aide">
          Courbe bleue : l'épargne du foyer va d'abord à la précaution, aux crédits chers et aux projets en cours ;
          chaque projet arrivé à échéance libère son versement pour la poche 2. Courbe orange : versement constant de{' '}
          {eur(p2.epargne_mensuelle_necessaire_eur)}/mois qui atteint exactement la cible.
        </p>
        <ProjectionChart points={p2.projection} />
        <details>
          <summary>Voir les valeurs</summary>
          <div className="defilement">
            <table className="tableau compact">
              <thead>
                <tr>
                  <th scope="col">Année</th>
                  <th scope="col" className="num">Épargne du foyer</th>
                  <th scope="col" className="num">Versement constant</th>
                  <th scope="col" className="num">Cible</th>
                </tr>
              </thead>
              <tbody>
                {p2.projection.map((pt) => (
                  <tr key={pt.annee}>
                    <td>{pt.annee}</td>
                    <td className="num">{eur(pt.capital_flux_disponible)}</td>
                    <td className="num">{eur(pt.capital_epargne_necessaire)}</td>
                    <td className="num">{eur(pt.cible)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </details>
        {p2.leviers.length > 0 && (
          <>
            <h3>Leviers pour atteindre l'objectif</h3>
            <ul className="leviers">
              {p2.leviers.map((l) => (
                <li key={l.titre}>
                  <strong>{l.titre}</strong>
                  <span>{l.detail}</span>
                </li>
              ))}
            </ul>
          </>
        )}
      </section>

      <section className="bloc">
        <h2>Phase « revenus passifs »</h2>
        <div className="deux-colonnes">
          <dl className="liste-def">
            <dt>Dérisquage</dt>
            <dd>
              dans {r.phase_revenus.annee_derisquage} ans → profil{' '}
              {r.phase_revenus.profil_phase === 'dynamique' ? 'Dynamique' : r.phase_revenus.profil_phase === 'equilibre' ? 'Équilibré' : 'Prudent'}
            </dd>
            <dt>Retrait annuel</dt>
            <dd>
              {eur(r.phase_revenus.retrait_annuel_eur)} <span className="muet">({eur(r.phase_revenus.retrait_mensuel_eur)}/mois)</span>
            </dd>
            <dt>Coussin fonds euros</dt>
            <dd>{eur(r.phase_revenus.coussin_fonds_euros_eur)}</dd>
          </dl>
          <div>
            <h3>Ordre des sources (coût fiscal croissant)</h3>
            <ol className="etapes">
              {r.phase_revenus.ordre_sources.map((s) => (
                <li key={s}>{s}</li>
              ))}
            </ol>
          </div>
        </div>
        <ul className="puces">
          {r.phase_revenus.regles.map((s) => (
            <li key={s}>{s}</li>
          ))}
        </ul>
      </section>
    </div>
  )
}
