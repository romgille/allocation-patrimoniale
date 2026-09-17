import type { Allocation } from '../bindings/Allocation'
import type { Hypotheses } from '../bindings/Hypotheses'
import type { PalierGlide } from '../bindings/PalierGlide'
import type { ProfilParams } from '../bindings/ProfilParams'
import { pct } from '../format'
import { LIGNES } from '../lignes'
import { NumberField } from './fields'

type Props = {
  h: Hypotheses
  defauts: Hypotheses | null
  onChange: (h: Hypotheses) => void
}

type CleNombre = { [K in keyof Hypotheses]: Hypotheses[K] extends number ? K : never }[keyof Hypotheses]
type CleProfil = 'prudent' | 'equilibre' | 'dynamique'

const PROFILS: { cle: CleProfil; titre: string }[] = [
  { cle: 'prudent', titre: 'Prudent' },
  { cle: 'equilibre', titre: 'Équilibré' },
  { cle: 'dynamique', titre: 'Dynamique' },
]

export function HypothesesForm({ h, defauts, onChange }: Props) {
  const set = <K extends keyof Hypotheses>(k: K, v: Hypotheses[K]) => onChange({ ...h, [k]: v })

  const eurF = (k: CleNombre, label: string, aide?: string) => (
    <NumberField key={k} label={label} unite="€" value={h[k]} onChange={(v) => set(k, v)} min={0} aide={aide} />
  )
  const pctF = (k: CleNombre, label: string, aide?: string) => (
    <NumberField key={k} label={label} unite="%" value={h[k]} facteur={100} onChange={(v) => set(k, v)} min={0} max={100} aide={aide} />
  )
  const nbF = (k: CleNombre, label: string, unite: string, entier = false) => (
    <NumberField key={k} label={label} unite={unite} value={h[k]} onChange={(v) => set(k, v)} min={0} entier={entier} />
  )

  const setProfil = (p: CleProfil, patch: Partial<ProfilParams>) => set(p, { ...h[p], ...patch })
  const setAlloc = (p: CleProfil, l: keyof Allocation, v: number) =>
    setProfil(p, { allocation: { ...h[p].allocation, [l]: v } })
  const setPalier = (i: number, patch: Partial<PalierGlide>) =>
    set('glide_path', h.glide_path.map((x, j) => (j === i ? { ...x, ...patch } : x)))

  return (
    <div className="hypotheses">
      <div className="barre-actions">
        <p className="aide">
          Valeurs par défaut issues de la spécification (septembre 2026). À vérifier au moment de l'utilisation — elles
          ne sont conservées que dans ce navigateur et dans vos exports.
        </p>
        {defauts && (
          <button type="button" className="bouton secondaire" onClick={() => onChange(defauts)}>
            Rétablir les valeurs par défaut
          </button>
        )}
      </div>

      <details open>
        <summary>Études et projets</summary>
        <div className="grille">
          {nbF('age_debut_etudes', 'Âge de début des études', 'ans', true)}
          {eurF('frais_scolarite_public', 'Frais de scolarité public / an')}
          {eurF('frais_scolarite_mixte', 'Frais de scolarité mixte / an')}
          {eurF('frais_scolarite_prive', 'Frais de scolarité privé / an')}
          {eurF('cout_logement_annuel', 'Logement + vie hors domicile / an')}
          {pctF('inflation_etudes', 'Inflation des coûts d’études')}
          {pctF('inflation_projets', 'Inflation appliquée aux projets')}
          {pctF('rendement_actions', 'Rendement actions (nominal, net de frais)')}
          {pctF('rendement_securise', 'Rendement sécurisé')}
        </div>
        <h4>Glide path</h4>
        <div className="defilement">
          <table className="tableau compact">
            <thead>
              <tr>
                <th scope="col">Horizon ≥ (ans)</th>
                <th scope="col">Part actions</th>
                <th scope="col">Part sécurisée</th>
              </tr>
            </thead>
            <tbody>
              {h.glide_path.map((p, i) => (
                <tr key={i}>
                  <td>
                    <NumberField label="" value={p.horizon_min_ans} entier min={0} onChange={(v) => setPalier(i, { horizon_min_ans: v })} />
                  </td>
                  <td>
                    <NumberField label="" unite="%" facteur={100} value={p.part_actions} min={0} max={100} onChange={(v) => setPalier(i, { part_actions: v })} />
                  </td>
                  <td className="num">{pct(1 - p.part_actions, 0)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </details>

      <details open>
        <summary>Profils de la poche 2</summary>
        <div className="defilement">
          <table className="tableau compact">
            <thead>
              <tr>
                <th scope="col">Ligne</th>
                {PROFILS.map((p) => (
                  <th scope="col" key={p.cle}>
                    {p.titre}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {LIGNES.map((l) => (
                <tr key={l.cle}>
                  <th scope="row">{l.libelle}</th>
                  {PROFILS.map((p) => (
                    <td key={p.cle}>
                      <NumberField label="" unite="%" facteur={100} min={0} max={100} value={h[p.cle].allocation[l.cle]} onChange={(v) => setAlloc(p.cle, l.cle, v)} />
                    </td>
                  ))}
                </tr>
              ))}
              <tr className="total">
                <th scope="row">Total</th>
                {PROFILS.map((p) => {
                  const t = LIGNES.reduce((s, l) => s + h[p.cle].allocation[l.cle], 0)
                  return (
                    <td key={p.cle} className={`num ${Math.abs(t - 1) > 0.001 ? 'erreur' : ''}`}>
                      {pct(t)}
                    </td>
                  )
                })}
              </tr>
              <tr>
                <th scope="row">Rendement attendu</th>
                {PROFILS.map((p) => (
                  <td key={p.cle}>
                    <NumberField label="" unite="%" facteur={100} min={-100} max={100} value={h[p.cle].rendement} onChange={(v) => setProfil(p.cle, { rendement: v })} />
                  </td>
                ))}
              </tr>
              <tr>
                <th scope="row">Perte max plausible (12 mois)</th>
                {PROFILS.map((p) => (
                  <td key={p.cle}>
                    <NumberField label="" unite="%" facteur={100} min={-100} max={0} value={h[p.cle].perte_max} onChange={(v) => setProfil(p.cle, { perte_max: v })} />
                  </td>
                ))}
              </tr>
            </tbody>
          </table>
        </div>
        <div className="grille">
          {nbF('seuil_score_prudent', 'Score ≤ … → Prudent', 'pts')}
          {nbF('seuil_score_dynamique', 'Score > … → Dynamique', 'pts')}
          {nbF('h_fire_court', 'Horizon court (< … ans)', 'ans', true)}
          {nbF('h_fire_long', 'Horizon long (> … ans)', 'ans', true)}
        </div>
      </details>

      <details>
        <summary>Précaution et dettes</summary>
        <div className="grille">
          {nbF('precaution_mois_min', 'Précaution minimale (RP payée, revenus stables)', 'mois')}
          {nbF('precaution_mois_defaut', 'Précaution par défaut', 'mois')}
          {nbF('precaution_mois_max', 'Précaution maximale (revenus instables)', 'mois')}
          {pctF('seuil_taux_dette', 'Rembourser en priorité les crédits au-delà de')}
          {pctF('endettement_max', 'Endettement maximal pour le levier immobilier')}
        </div>
      </details>

      <details>
        <summary>Plafonds et planchers</summary>
        <div className="grille">
          {nbF('age_regle_actions', 'Règle « … − âge » pour la part actions', '', true)}
          {pctF('actions_max_tolerance_faible', 'Actions max si tolérance ≤ 2')}
          {pctF('plancher_fonds_euros', 'Plancher fonds euros / obligations')}
          {pctF('plancher_fonds_euros_horizon_court', 'Plancher fonds euros (horizon court)')}
          {pctF('or_min', 'Or minimum')}
          {pctF('or_max', 'Or maximum')}
          {pctF('crowdfunding_max', 'Crowdfunding maximum')}
          {pctF('actions_directes_max', 'Actions en direct maximum')}
          {pctF('satellites_max', 'Satellites cumulés maximum')}
          {pctF('crypto_max', 'Crypto maximum')}
          {pctF('immo_max', 'Immobilier total maximum')}
          {pctF('immo_max_gestion_elevee', 'Immobilier max (temps de gestion élevé)')}
          {pctF('seuil_concentration_immo', 'Seuil de concentration du locatif')}
          {pctF('tolerance_reequilibrage', 'Tolérance avant rééquilibrage')}
        </div>
      </details>

      <details>
        <summary>Fiscalité et enveloppes (2026)</summary>
        <div className="grille">
          {eurF('plafond_pea', 'Plafond de versements PEA')}
          {pctF('pfu', 'PFU (flat tax)')}
          {pctF('prelevements_sociaux', 'Prélèvements sociaux')}
          {eurF('abattement_av_seul', 'Abattement AV > 8 ans (seul)')}
          {eurF('abattement_av_couple', 'Abattement AV > 8 ans (couple)')}
          {eurF('abattement_donation', 'Abattement donation parent → enfant')}
          {pctF('plafond_per_revenus', 'Plafond de déduction PER (revenus)')}
        </div>
      </details>

      <details>
        <summary>Phase revenus passifs</summary>
        <div className="grille">
          {nbF('annees_derisquage', 'Baisser le profil … ans avant l’échéance', 'ans', true)}
          {nbF('annees_coussin', 'Coussin en fonds euros', 'ans de retraits', true)}
        </div>
      </details>
    </div>
  )
}
