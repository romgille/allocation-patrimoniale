import type { Depense } from '../bindings/Depense'
import type { Dette } from '../bindings/Dette'
import type { ObjetCredit } from '../bindings/ObjetCredit'
import type { PosteDepense } from '../bindings/PosteDepense'
import type { Questionnaire } from '../bindings/Questionnaire'
import type { Revenu } from '../bindings/Revenu'
import { eur } from '../format'
import { mensualiteTheorique } from '../credit'
import { NumberField, SelectField, TextField, Toggle } from './fields'

export const OBJETS_CREDIT: { value: ObjetCredit; label: string }[] = [
  { value: 'residence_principale', label: 'Résidence principale' },
  { value: 'locatif', label: 'Immobilier locatif' },
  { value: 'consommation', label: 'Consommation' },
  { value: 'autre', label: 'Autre' },
]

/** Libellés des postes : `Record` garantit que tous les postes Rust sont couverts. */
export const POSTES: Record<PosteDepense, { libelle: string; exemple: string }> = {
  logement: { libelle: 'Logement', exemple: 'Loyer, charges, énergie' },
  alimentation: { libelle: 'Alimentation', exemple: 'Courses, cantine adultes' },
  transport: { libelle: 'Transport', exemple: 'Carburant, entretien, pass' },
  enfants: { libelle: 'Enfants', exemple: 'Garde, cantine, activités' },
  sante_assurances: { libelle: 'Santé et assurances', exemple: 'Mutuelle, assurances' },
  abonnements: { libelle: 'Abonnements', exemple: 'Box, mobiles, streaming' },
  loisirs: { libelle: 'Loisirs et vacances', exemple: 'Sorties, vacances lissées' },
  impots: { libelle: 'Impôts', exemple: 'IR, taxe foncière (mensualisés)' },
  autre: { libelle: 'Autres', exemple: 'Cadeaux, divers' },
}
const ORDRE_POSTES = Object.keys(POSTES) as PosteDepense[]

const somme = (xs: number[]) => xs.reduce((s, x) => s + x, 0)

type Props = { q: Questionnaire; onChange: (q: Questionnaire) => void }

export function BudgetForm({ q, onChange }: Props) {
  const set = <K extends keyof Questionnaire>(k: K, v: Questionnaire[K]) => onChange({ ...q, [k]: v })

  const revenus = somme(q.adultes.flatMap((a) => a.revenus.map((r) => r.montant_mensuel)))
  const depenses = somme(q.depenses.map((d) => d.montant_mensuel))
  // Seuls les crédits du foyer pèsent sur sa capacité d'épargne : ceux des SCI sont
  // remboursés par les loyers (l'éventuel effort à combler apparaît dans les résultats).
  const mensualites = somme(q.dettes.filter((d) => d.sci_id === null).map((d) => d.mensualite))
  const mensualitesScis = somme(q.dettes.filter((d) => d.sci_id !== null).map((d) => d.mensualite))
  const capacite = revenus - depenses - mensualites

  const majRevenus = (i: number, revenus: Revenu[]) =>
    set(
      'adultes',
      q.adultes.map((a, j) => (j === i ? { ...a, revenus } : a)),
    )

  // Les dépenses restent une liste plate ; on garde l'index d'origine pour les modifier.
  const indexees = q.depenses.map((d, i) => ({ d, i }))
  const majDepense = (i: number, patch: Partial<Depense>) =>
    set(
      'depenses',
      q.depenses.map((d, j) => (j === i ? { ...d, ...patch } : d)),
    )

  return (
    <>
      <h3>Revenus nets mensuels</h3>
      <div className="grille-cartes">
        {q.adultes.map((a, i) => (
          <fieldset key={i} className="carte-saisie">
            <legend>
              {a.prenom || `Adulte ${i + 1}`} <span className="muet">· {eur(somme(a.revenus.map((r) => r.montant_mensuel)))}</span>
            </legend>
            {a.revenus.map((r, k) => (
              <div key={k} className="ligne-saisie">
                <TextField
                  label="Libellé"
                  value={r.libelle}
                  onChange={(v) => majRevenus(i, a.revenus.map((x, m) => (m === k ? { ...x, libelle: v } : x)))}
                />
                <NumberField
                  label="Montant"
                  unite="€"
                  min={0}
                  value={r.montant_mensuel}
                  onChange={(v) => majRevenus(i, a.revenus.map((x, m) => (m === k ? { ...x, montant_mensuel: v } : x)))}
                />
                <button
                  type="button"
                  className="icone danger"
                  aria-label={`Retirer ${r.libelle}`}
                  onClick={() => majRevenus(i, a.revenus.filter((_, m) => m !== k))}
                >
                  ×
                </button>
              </div>
            ))}
            <button
              type="button"
              className="lien"
              onClick={() => majRevenus(i, [...a.revenus, { libelle: 'Autre revenu', montant_mensuel: 0 }])}
            >
              + Ajouter un revenu
            </button>
          </fieldset>
        ))}
      </div>
      <p className="aide">Primes, 13e mois, allocations : lissez-les sur 12 mois.</p>

      <h3>Dépenses courantes du foyer</h3>
      <p className="aide">Hors mensualités de crédit, saisies juste en dessous.</p>
      <div className="grille-cartes">
        {ORDRE_POSTES.map((poste) => {
          const lignes = indexees.filter((x) => x.d.poste === poste)
          const total = somme(lignes.map((x) => x.d.montant_mensuel))
          return (
            <fieldset key={poste} className="carte-saisie">
              <legend>
                {POSTES[poste].libelle} <span className="muet">· {eur(total)}</span>
              </legend>
              {lignes.map(({ d, i }) => (
                <div key={i} className="ligne-saisie">
                  <TextField label="Libellé" value={d.libelle} onChange={(v) => majDepense(i, { libelle: v })} />
                  <NumberField label="Montant" unite="€" min={0} value={d.montant_mensuel} onChange={(v) => majDepense(i, { montant_mensuel: v })} />
                  <button
                    type="button"
                    className="icone danger"
                    aria-label={`Retirer ${d.libelle}`}
                    onClick={() => set('depenses', q.depenses.filter((_, j) => j !== i))}
                  >
                    ×
                  </button>
                </div>
              ))}
              <button
                type="button"
                className="lien"
                onClick={() => set('depenses', [...q.depenses, { poste, libelle: POSTES[poste].exemple, montant_mensuel: 0 }])}
              >
                + Ajouter
              </button>
            </fieldset>
          )
        })}
      </div>

      <h3>Crédits en cours</h3>
      <DettesForm dettes={q.dettes} scis={q.scis} onChange={(d) => set('dettes', d)} />

      <h3>Capacité d'épargne</h3>
      <div className="recap-budget" aria-live="polite">
        <span>Revenus</span>
        <strong>{eur(revenus)}</strong>
        <span>− Dépenses courantes</span>
        <strong>{eur(depenses)}</strong>
        <span>− Mensualités de crédit du foyer</span>
        <strong>{eur(mensualites)}</strong>
        <span className="total">= Capacité d'épargne</span>
        <strong className={`total ${capacite < 0 ? 'ko' : ''}`}>{eur(capacite)}</strong>
      </div>
      {mensualitesScis > 0 && (
        <p className="aide">
          {eur(mensualitesScis)} de mensualités sont portées par vos SCI et remboursées par leurs loyers. Si leur
          trésorerie ne suffit pas, l'effort restant à votre charge apparaît dans les résultats.
        </p>
      )}
      <section className="grille">
        <Toggle
          label="Imposer un autre montant d'épargne"
          value={q.epargne_mensuelle_forcee !== null}
          onChange={(v) => set('epargne_mensuelle_forcee', v ? Math.max(0, Math.round(capacite)) : null)}
          aide="Utile si une partie de la marge sert déjà à autre chose."
        />
        {q.epargne_mensuelle_forcee !== null && (
          <NumberField
            label="Épargne mensuelle retenue"
            unite="€/mois"
            min={0}
            value={q.epargne_mensuelle_forcee}
            onChange={(v) => set('epargne_mensuelle_forcee', v)}
          />
        )}
      </section>
    </>
  )
}

function DettesForm({
  dettes,
  scis,
  onChange,
}: {
  dettes: Dette[]
  scis: Questionnaire['scis']
  onChange: (d: Dette[]) => void
}) {
  const maj = (i: number, patch: Partial<Dette>) => onChange(dettes.map((d, j) => (j === i ? { ...d, ...patch } : d)))
  return (
    <>
      {dettes.length === 0 && <p className="vide">Aucun crédit.</p>}
      {dettes.map((d, i) => {
        const theorique = mensualiteTheorique(d.restant_du, d.taux_pct / 100, d.duree_restante_mois)
        const ecart = theorique > 0 ? Math.abs(d.mensualite - theorique) / theorique : 0
        return (
          <fieldset key={i} className="carte-saisie">
            <legend>
              {d.libelle || `Crédit ${i + 1}`}
              <button type="button" className="lien danger" onClick={() => onChange(dettes.filter((_, j) => j !== i))}>
                Retirer
              </button>
            </legend>
            <div className="grille">
              <TextField label="Libellé" value={d.libelle} onChange={(v) => maj(i, { libelle: v })} />
              <SelectField
                label="Objet"
                value={d.objet}
                options={OBJETS_CREDIT}
                onChange={(v) => maj(i, { objet: v })}
                aide="Un crédit locatif à taux bas est un levier, pas une dette à solder."
              />
              <NumberField label="Taux" unite="%" value={d.taux_pct} onChange={(v) => maj(i, { taux_pct: v })} min={0} max={30} />
              <NumberField label="Restant dû" unite="€" value={d.restant_du} onChange={(v) => maj(i, { restant_du: v })} min={0} />
              <NumberField
                label="Durée restante"
                unite="mois"
                value={d.duree_restante_mois}
                onChange={(v) => maj(i, { duree_restante_mois: v })}
                min={0}
                max={600}
                entier
                aide="0 si inconnue. Renseignée, la mensualité revient à l'épargne à la fin du crédit."
              />
              <NumberField
                label="Mensualité"
                unite="€/mois"
                value={d.mensualite}
                onChange={(v) => maj(i, { mensualite: v })}
                min={0}
                aide={
                  theorique > 0 && ecart > 0.05 ? (
                    <span className="alerte-champ">
                      Avec ce taux et cette durée, elle devrait être d'environ {eur(theorique)}.{' '}
                      <button type="button" className="lien" onClick={() => maj(i, { mensualite: Math.round(theorique) })}>
                        Corriger
                      </button>
                    </span>
                  ) : undefined
                }
              />
              {scis.length > 0 && (
                <SelectField
                  label="Porté par"
                  value={d.sci_id ?? ''}
                  options={[{ value: '', label: 'Le foyer' }, ...scis.map((s) => ({ value: s.id, label: s.nom || 'SCI' }))]}
                  onChange={(v) => maj(i, { sci_id: v === '' ? null : v })}
                  aide="Un crédit de SCI est remboursé par ses loyers, pas par l'épargne du foyer."
                />
              )}
            </div>
          </fieldset>
        )
      })}
      <button
        type="button"
        className="bouton secondaire"
        onClick={() =>
          onChange([
            ...dettes,
            {
              libelle: `Crédit ${dettes.length + 1}`,
              taux_pct: 0,
              restant_du: 0,
              mensualite: 0,
              duree_restante_mois: 0,
              objet: 'autre',
              sci_id: null,
            },
          ])
        }
      >
        + Ajouter un crédit
      </button>
    </>
  )
}
