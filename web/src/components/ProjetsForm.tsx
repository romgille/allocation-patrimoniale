import type { Enfant } from '../bindings/Enfant'
import type { Priorite } from '../bindings/Priorite'
import type { Projet } from '../bindings/Projet'
import type { Questionnaire } from '../bindings/Questionnaire'
import type { TypeEcole } from '../bindings/TypeEcole'
import { NumberField, Segmented, SelectField, TextField, Toggle } from './fields'

export const PRIORITES: { value: Priorite; label: string }[] = [
  { value: 'essentiel', label: 'Essentiel' },
  { value: 'important', label: 'Important' },
  { value: 'souhaitable', label: 'Souhaitable' },
]

const IDEES = ['Changer de voiture', 'Travaux', 'Voyage en famille', 'Apport résidence principale', 'Mariage', 'Permis de conduire']

type Props = { q: Questionnaire; onChange: (q: Questionnaire) => void }

export function ProjetsForm({ q, onChange }: Props) {
  const set = <K extends keyof Questionnaire>(k: K, v: Questionnaire[K]) => onChange({ ...q, [k]: v })
  return (
    <>
      <h3>Enfants — études</h3>
      <EnfantsForm enfants={q.enfants} onChange={(e) => set('enfants', e)} />
      <h3>Projets du foyer</h3>
      <p className="aide">
        Chaque projet a sa propre poche datée. Quand l'épargne ne suffit pas, les études et les projets essentiels
        passent en premier, puis les importants, puis les souhaitables.
      </p>
      <ProjetsListe projets={q.projets} onChange={(p) => set('projets', p)} />
    </>
  )
}

function ProjetsListe({ projets, onChange }: { projets: Projet[]; onChange: (p: Projet[]) => void }) {
  const annee = new Date().getFullYear()
  const maj = (i: number, patch: Partial<Projet>) => onChange(projets.map((p, j) => (j === i ? { ...p, ...patch } : p)))
  const suggestion = IDEES.find((x) => !projets.some((p) => p.libelle === x)) ?? `Projet ${projets.length + 1}`
  return (
    <>
      {projets.length === 0 && <p className="vide">Aucun projet daté.</p>}
      {projets.map((p, i) => (
        <fieldset key={i} className="carte-saisie">
          <legend>
            {p.libelle || `Projet ${i + 1}`}
            <button type="button" className="lien danger" onClick={() => onChange(projets.filter((_, j) => j !== i))}>
              Retirer
            </button>
          </legend>
          <div className="grille">
            <TextField label="Projet" value={p.libelle} onChange={(v) => maj(i, { libelle: v })} />
            <NumberField label="Montant (en euros d'aujourd'hui)" unite="€" min={0} value={p.montant} onChange={(v) => maj(i, { montant: v })} />
            <NumberField
              label="Échéance"
              unite="ans"
              entier
              min={0}
              max={60}
              value={p.dans_ans}
              onChange={(v) => maj(i, { dans_ans: v })}
              aide={p.dans_ans === 0 ? 'Cette année' : `Vers ${annee + p.dans_ans}`}
            />
            <Segmented<Priorite> label="Priorité" value={p.priorite} options={PRIORITES} onChange={(v) => maj(i, { priorite: v })} />
            <NumberField
              label="Déjà mis de côté"
              unite="€"
              min={0}
              value={p.capital_deja_affecte}
              onChange={(v) => maj(i, { capital_deja_affecte: v })}
            />
          </div>
        </fieldset>
      ))}
      <button
        type="button"
        className="bouton secondaire"
        onClick={() =>
          onChange([...projets, { libelle: suggestion, montant: 5000, dans_ans: 3, priorite: 'important', capital_deja_affecte: 0 }])
        }
      >
        + Ajouter un projet
      </button>
    </>
  )
}

function EnfantsForm({ enfants, onChange }: { enfants: Enfant[]; onChange: (e: Enfant[]) => void }) {
  const maj = (i: number, patch: Partial<Enfant>) => onChange(enfants.map((e, j) => (j === i ? { ...e, ...patch } : e)))
  return (
    <>
      {enfants.length === 0 && <p className="vide">Aucun enfant : pas de poche études.</p>}
      {enfants.map((e, i) => (
        <fieldset key={i} className="carte-saisie">
          <legend>
            {e.prenom || `Enfant ${i + 1}`}
            <button type="button" className="lien danger" onClick={() => onChange(enfants.filter((_, j) => j !== i))}>
              Retirer
            </button>
          </legend>
          <div className="grille">
            <TextField label="Prénom" value={e.prenom} onChange={(v) => maj(i, { prenom: v })} />
            <NumberField label="Âge" unite="ans" value={e.age} onChange={(v) => maj(i, { age: v })} entier min={0} max={30} />
            <SelectField<TypeEcole>
              label="Type d'école"
              value={e.type_ecole}
              onChange={(v) => maj(i, { type_ecole: v })}
              options={[
                { value: 'public', label: 'Public' },
                { value: 'mixte', label: 'Mixte' },
                { value: 'prive', label: 'Privé' },
              ]}
            />
            <NumberField
              label="Années à financer"
              unite="ans"
              value={e.duree_etudes}
              onChange={(v) => maj(i, { duree_etudes: v })}
              entier
              min={1}
              max={10}
              aide="Prépa + école."
            />
            <Toggle label="Logement hors du domicile" value={e.logement_etudiant} onChange={(v) => maj(i, { logement_etudiant: v })} />
            <NumberField
              label="Capital déjà mis de côté"
              unite="€"
              value={e.capital_deja_affecte}
              onChange={(v) => maj(i, { capital_deja_affecte: v })}
              min={0}
            />
          </div>
        </fieldset>
      ))}
      <button
        type="button"
        className="bouton secondaire"
        onClick={() =>
          onChange([
            ...enfants,
            {
              prenom: `Enfant ${enfants.length + 1}`,
              age: 0,
              type_ecole: 'mixte',
              logement_etudiant: true,
              duree_etudes: 5,
              capital_deja_affecte: 0,
            },
          ])
        }
      >
        + Ajouter un enfant
      </button>
    </>
  )
}
