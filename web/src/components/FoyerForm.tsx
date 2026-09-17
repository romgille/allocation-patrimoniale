import type { Adulte } from '../bindings/Adulte'
import type { Questionnaire } from '../bindings/Questionnaire'
import type { StatutRp } from '../bindings/StatutRp'
import type { TempsGestion } from '../bindings/TempsGestion'
import type { Tmi } from '../bindings/Tmi'
import { NumberField, Segmented, SelectField, TextField, Toggle } from './fields'

export function nouvelAdulte(n: number): Adulte {
  return {
    prenom: `Adulte ${n}`,
    age: 35,
    revenus: [{ libelle: 'Salaire net', montant_mensuel: 0 }],
    stab_revenus: 3,
    tol_risque: 3,
    temps_gestion: 'faible',
    abondement_employeur: false,
  }
}

type Props = { q: Questionnaire; onChange: (q: Questionnaire) => void }

export function FoyerForm({ q, onChange }: Props) {
  const set = <K extends keyof Questionnaire>(k: K, v: Questionnaire[K]) => onChange({ ...q, [k]: v })
  const majAdulte = (i: number, patch: Partial<Adulte>) =>
    set(
      'adultes',
      q.adultes.map((a, j) => (j === i ? { ...a, ...patch } : a)),
    )

  return (
    <>
      <p className="aide intro">
        Chaque adulte répond pour lui-même. Pour le profil de risque, l'outil retient la réponse la plus prudente
        du foyer et signale les écarts importants.
      </p>
      {q.adultes.map((a, i) => (
        <fieldset key={i} className="carte-saisie">
          <legend>
            {a.prenom || `Adulte ${i + 1}`}
            {q.adultes.length > 1 && (
              <button
                type="button"
                className="lien danger"
                onClick={() => set('adultes', q.adultes.filter((_, j) => j !== i))}
              >
                Retirer
              </button>
            )}
          </legend>
          <div className="grille">
            <TextField label="Prénom" value={a.prenom} onChange={(v) => majAdulte(i, { prenom: v })} />
            <NumberField label="Âge" unite="ans" value={a.age} onChange={(v) => majAdulte(i, { age: v })} entier min={16} max={100} />
            <Segmented<number>
              label="Stabilité des revenus"
              value={a.stab_revenus}
              onChange={(v) => majAdulte(i, { stab_revenus: v })}
              aide="Variable = indépendant ; Stable = CDI ou fonctionnaire."
              options={[
                { value: 1, label: 'Variable' },
                { value: 2, label: 'Intermédiaire' },
                { value: 3, label: 'Stable' },
              ]}
            />
            <Segmented<number>
              label="Tolérance au risque"
              value={a.tol_risque}
              onChange={(v) => majAdulte(i, { tol_risque: v })}
              options={[1, 2, 3, 4, 5].map((n) => ({ value: n, label: String(n) }))}
              aide="1 = « je vends si le marché perd 20 % » … 5 = « je renforce »."
            />
            <Segmented<TempsGestion>
              label="Temps disponible pour gérer"
              value={a.temps_gestion}
              onChange={(v) => majAdulte(i, { temps_gestion: v })}
              options={[
                { value: 'faible', label: 'Faible' },
                { value: 'moyen', label: 'Moyen' },
                { value: 'eleve', label: 'Élevé' },
              ]}
            />
            <Toggle
              label="Abondement employeur (PEE / PER collectif)"
              value={a.abondement_employeur}
              onChange={(v) => majAdulte(i, { abondement_employeur: v })}
            />
          </div>
        </fieldset>
      ))}
      {q.adultes.length < 6 && (
        <button type="button" className="bouton secondaire" onClick={() => set('adultes', [...q.adultes, nouvelAdulte(q.adultes.length + 1)])}>
          + Ajouter un adulte
        </button>
      )}

      <h3>Situation du foyer</h3>
      <section className="grille">
        <Segmented<Tmi>
          label="Tranche marginale d'imposition du foyer"
          value={q.tmi}
          onChange={(v) => set('tmi', v)}
          options={(['0', '11', '30', '41', '45'] as const).map((t) => ({ value: t, label: `${t} %` }))}
        />
        <SelectField<StatutRp>
          label="Résidence principale"
          value={q.rp}
          onChange={(v) => set('rp', v)}
          options={[
            { value: 'locataire', label: 'Locataire' },
            { value: 'proprietaire_avec_credit', label: 'Propriétaire, crédit en cours' },
            { value: 'proprietaire_payee', label: 'Propriétaire, payée (ou quasi)' },
          ]}
        />
        <Toggle label="Foyer éligible au LEP" value={q.lep_eligible} onChange={(v) => set('lep_eligible', v)} />
      </section>
    </>
  )
}
