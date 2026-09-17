import type { Avoirs } from '../bindings/Avoirs'
import type { Contraintes } from '../bindings/Contraintes'
import type { Questionnaire } from '../bindings/Questionnaire'
import { BudgetForm } from './BudgetForm'
import { FoyerForm } from './FoyerForm'
import { NumberField, Toggle } from './fields'
import { ProjetsForm } from './ProjetsForm'

export type Etape = 'foyer' | 'budget' | 'projets' | 'objectifs' | 'patrimoine' | 'preferences'

export const ETAPES: { id: Etape; titre: string }[] = [
  { id: 'foyer', titre: 'Foyer' },
  { id: 'budget', titre: 'Budget' },
  { id: 'projets', titre: 'Enfants & projets' },
  { id: 'objectifs', titre: 'Revenus passifs' },
  { id: 'patrimoine', titre: 'Patrimoine' },
  { id: 'preferences', titre: 'Préférences' },
]

type Props = {
  etape: Etape
  q: Questionnaire
  onChange: (q: Questionnaire) => void
}

export function QuestionnaireForm({ etape, q, onChange }: Props) {
  const set = <K extends keyof Questionnaire>(k: K, v: Questionnaire[K]) => onChange({ ...q, [k]: v })
  const setAvoir = <K extends keyof Avoirs>(k: K, v: Avoirs[K]) => set('avoirs', { ...q.avoirs, [k]: v })
  const setC = <K extends keyof Contraintes>(k: K, v: Contraintes[K]) =>
    set('contraintes', { ...q.contraintes, [k]: v })
  const ageMax = Math.max(0, ...q.adultes.map((a) => a.age))

  switch (etape) {
    case 'foyer':
      return <FoyerForm q={q} onChange={onChange} />

    case 'budget':
      return <BudgetForm q={q} onChange={onChange} />

    case 'projets':
      return <ProjetsForm q={q} onChange={onChange} />

    case 'objectifs':
      return (
        <section className="grille">
          <NumberField
            label="Dans combien d'années le foyer veut-il réduire son temps de travail ?"
            unite="ans"
            value={q.h_fire}
            onChange={(v) => set('h_fire', v)}
            entier
            min={0}
            max={60}
            aide={`Vers ${new Date().getFullYear() + q.h_fire}${ageMax > 0 ? ` (l'aîné aura ${ageMax + q.h_fire} ans)` : ''}.`}
          />
          <NumberField
            label="Revenu passif mensuel visé pour le foyer (net)"
            unite="€/mois"
            value={q.r_cible}
            onChange={(v) => set('r_cible', v)}
            min={0}
          />
          <NumberField
            label="Taux de retrait"
            unite="%"
            value={q.taux_retrait_pct}
            onChange={(v) => set('taux_retrait_pct', v)}
            min={0.1}
            max={10}
            aide="Prudent : 3 à 3,5 %. 4 % est agressif."
          />
          <NumberField
            label="Autres revenus passifs déjà acquis"
            unite="€/mois"
            value={q.autres_revenus_passifs}
            onChange={(v) => set('autres_revenus_passifs', v)}
            min={0}
            aide="Hors locatif (saisi à l'étape Patrimoine)."
          />
        </section>
      )

    case 'patrimoine':
      return (
        <>
          <h3>Épargne disponible</h3>
          <section className="grille">
            <NumberField label="Livrets (A, LDDS, LEP…)" unite="€" value={q.avoirs.livrets} onChange={(v) => setAvoir('livrets', v)} min={0} aide="Épargne de précaution, tous membres confondus." />
            <NumberField
              label="Liquidités à investir"
              unite="€"
              value={q.avoirs.liquidites_a_investir}
              onChange={(v) => setAvoir('liquidites_a_investir', v)}
              min={0}
              aide="Excédent de compte courant, cash en attente sur PEA/CTO."
            />
          </section>
          <h3>Placements du foyer (hors sommes déjà affectées aux études et projets)</h3>
          <section className="grille">
            <NumberField label="ETF actions monde" unite="€" value={q.avoirs.etf_monde} onChange={(v) => setAvoir('etf_monde', v)} min={0} />
            <NumberField label="Fonds euros / obligations" unite="€" value={q.avoirs.fonds_euros_obligations} onChange={(v) => setAvoir('fonds_euros_obligations', v)} min={0} />
            <NumberField label="SCPI" unite="€" value={q.avoirs.scpi} onChange={(v) => setAvoir('scpi', v)} min={0} />
            <NumberField label="Or" unite="€" value={q.avoirs.or} onChange={(v) => setAvoir('or', v)} min={0} />
            <NumberField label="Actions en direct" unite="€" value={q.avoirs.actions_directes} onChange={(v) => setAvoir('actions_directes', v)} min={0} />
            <NumberField label="Crowdfunding immobilier" unite="€" value={q.avoirs.crowdfunding_immo} onChange={(v) => setAvoir('crowdfunding_immo', v)} min={0} />
            <NumberField label="Crowdfunding énergies renouvelables" unite="€" value={q.avoirs.crowdfunding_enr} onChange={(v) => setAvoir('crowdfunding_enr', v)} min={0} />
            <NumberField label="Crypto" unite="€" value={q.avoirs.crypto} onChange={(v) => setAvoir('crypto', v)} min={0} />
            <NumberField label="Private equity" unite="€" value={q.avoirs.private_equity} onChange={(v) => setAvoir('private_equity', v)} min={0} />
          </section>
          <h3>Immobilier locatif</h3>
          <section className="grille">
            <NumberField
              label="Valeur nette du locatif"
              unite="€"
              value={q.v_immo_loc}
              onChange={(v) => set('v_immo_loc', v)}
              min={0}
              aide="Valeur du bien − capital restant dû. Hors résidence principale."
            />
            <NumberField
              label="Cash-flow net mensuel"
              unite="€/mois"
              value={q.cf_immo}
              onChange={(v) => set('cf_immo', v)}
              aide="Après crédit, charges et impôts. Peut être négatif."
            />
          </section>
        </>
      )

    case 'preferences':
      return (
        <section className="grille">
          <Toggle label="Préférence ESG / ISR" value={q.contraintes.esg} onChange={(v) => setC('esg', v)} />
          <Toggle label="Refus de l'immobilier en direct" value={q.contraintes.refus_immo_direct} onChange={(v) => setC('refus_immo_direct', v)} />
          <Toggle label="Refus des cryptos" value={q.contraintes.refus_crypto} onChange={(v) => setC('refus_crypto', v)} />
          {!q.contraintes.refus_crypto && (
            <NumberField
              label="Part de crypto souhaitée"
              unite="%"
              value={q.contraintes.crypto_souhaitee_pct}
              onChange={(v) => setC('crypto_souhaitee_pct', v)}
              min={0}
              max={100}
              aide="Plafonnée à 3 % et prise sur la ligne actions en direct. 0 par défaut."
            />
          )}
        </section>
      )
  }
}
