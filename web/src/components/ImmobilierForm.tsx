import type { BienVendu } from '../bindings/BienVendu'
import type { Questionnaire } from '../bindings/Questionnaire'
import type { RegimeSci } from '../bindings/RegimeSci'
import type { Sci } from '../bindings/Sci'
import type { VenteImmobiliere } from '../bindings/VenteImmobiliere'
import { eur } from '../format'
import { NumberField, SelectField, Segmented, TextField } from './fields'

const REGIMES: { value: RegimeSci; label: string }[] = [
  { value: 'ir', label: "IR" },
  { value: 'is', label: 'IS' },
]

/** Identifiant stable, indépendant de la position dans la liste. */
function nouvelId(existants: string[]): string {
  for (let n = 1; ; n++) {
    const id = `sci-${n}`
    if (!existants.includes(id)) return id
  }
}

const cleBien = (b: BienVendu) => (b.type === 'sci' ? `sci:${b.id}` : 'direct')

type Props = { q: Questionnaire; onChange: (q: Questionnaire) => void }

export function ImmobilierForm({ q, onChange }: Props) {
  const set = <K extends keyof Questionnaire>(k: K, v: Questionnaire[K]) => onChange({ ...q, [k]: v })

  const majSci = (i: number, patch: Partial<Sci>) =>
    set('scis', q.scis.map((s, j) => (j === i ? { ...s, ...patch } : s)))

  const retirerSci = (i: number) => {
    const id = q.scis[i]?.id
    onChange({
      ...q,
      scis: q.scis.filter((_, j) => j !== i),
      // Les crédits et ventes qui visaient cette SCI deviendraient orphelins.
      dettes: q.dettes.map((d) => (d.sci_id === id ? { ...d, sci_id: null } : d)),
      ventes: q.ventes.filter((v) => !(v.bien.type === 'sci' && v.bien.id === id)),
    })
  }

  const majVente = (i: number, patch: Partial<VenteImmobiliere>) =>
    set('ventes', q.ventes.map((v, j) => (j === i ? { ...v, ...patch } : v)))

  const biens: { value: string; label: string }[] = [
    { value: 'direct', label: 'Locatif en direct' },
    ...q.scis.map((s) => ({ value: `sci:${s.id}`, label: s.nom || 'SCI' })),
  ]

  return (
    <>
      <h3>Immobilier locatif en direct</h3>
      <section className="grille">
        <NumberField
          label="Valeur nette du locatif"
          unite="€"
          value={q.v_immo_loc}
          onChange={(v) => set('v_immo_loc', v)}
          min={0}
          aide="Valeur du bien − capital restant dû. Hors résidence principale et hors SCI."
        />
        <NumberField
          label="Cash-flow net mensuel"
          unite="€/mois"
          value={q.cf_immo}
          onChange={(v) => set('cf_immo', v)}
          aide="Après crédit, charges et impôts. Peut être négatif."
        />
      </section>

      <h3>SCI</h3>
      <p className="aide">
        Pour une SCI, saisissez les loyers et les charges bruts : l'outil calcule lui-même l'impôt selon le régime.
        Les crédits de la SCI se rattachent à l'étape Budget.
      </p>
      {q.scis.length === 0 && <p className="vide">Aucune SCI.</p>}
      {q.scis.map((s, i) => {
        const credits = q.dettes.filter((d) => d.sci_id === s.id)
        const mensualites = credits.reduce((t, d) => t + d.mensualite, 0)
        return (
          <fieldset key={s.id} className="carte-saisie">
            <legend>
              {s.nom || `SCI ${i + 1}`}
              <button type="button" className="lien danger" onClick={() => retirerSci(i)}>
                Retirer
              </button>
            </legend>
            <div className="grille">
              <TextField label="Nom" value={s.nom} onChange={(v) => majSci(i, { nom: v })} />
              <Segmented
                label="Régime fiscal"
                value={s.regime}
                options={REGIMES}
                onChange={(v) => majSci(i, { regime: v })}
                aide={
                  s.regime === 'ir'
                    ? "Les loyers s'ajoutent à vos revenus et sont imposés à votre tranche marginale, plus les prélèvements sociaux. Le capital remboursé n'est pas déductible."
                    : "La société amortit le bien, ce qui réduit fortement l'impôt annuel. En contrepartie, les amortissements sont réintégrés dans la plus-value à la revente."
                }
              />
              <NumberField
                label="Quote-part du foyer"
                unite="%"
                value={s.part_foyer_pct}
                onChange={(v) => majSci(i, { part_foyer_pct: v })}
                min={0}
                max={100}
                aide="Part des parts sociales détenues par le foyer."
              />
              <NumberField label="Valeur des biens" unite="€" value={s.valeur_biens} onChange={(v) => majSci(i, { valeur_biens: v })} min={0} />
              <NumberField label="SCPI détenues par la SCI" unite="€" value={s.scpi} onChange={(v) => majSci(i, { scpi: v })} min={0} />
              <NumberField label="Loyers encaissés" unite="€/mois" value={s.loyers_mensuels} onChange={(v) => majSci(i, { loyers_mensuels: v })} min={0} />
              <NumberField
                label="Charges"
                unite="€/mois"
                value={s.charges_mensuelles}
                onChange={(v) => majSci(i, { charges_mensuelles: v })}
                min={0}
                aide="Taxe foncière, gestion, entretien. Hors crédit et hors impôt."
              />
              {s.regime === 'is' && (
                <NumberField
                  label="Part du résultat distribuée"
                  unite="%"
                  value={s.distribution_pct}
                  onChange={(v) => majSci(i, { distribution_pct: v })}
                  min={0}
                  max={100}
                  aide="Ce qui reste dans la société grossit votre patrimoine mais n'est pas un revenu passif."
                />
              )}
            </div>
            {credits.length > 0 && (
              <p className="aide">
                {credits.length} crédit{credits.length > 1 ? 's' : ''} rattaché{credits.length > 1 ? 's' : ''} ·{' '}
                {eur(mensualites)} par mois.
              </p>
            )}
          </fieldset>
        )
      })}
      <button
        type="button"
        className="bouton secondaire"
        onClick={() =>
          set('scis', [
            ...q.scis,
            {
              id: nouvelId(q.scis.map((s) => s.id)),
              nom: `SCI ${q.scis.length + 1}`,
              regime: 'is',
              part_foyer_pct: 100,
              valeur_biens: 0,
              scpi: 0,
              loyers_mensuels: 0,
              charges_mensuelles: 0,
              distribution_pct: 0,
            },
          ])
        }
      >
        + Ajouter une SCI
      </button>

      <h3>Ventes programmées</h3>
      <p className="aide">
        Le produit net de la vente — une fois le crédit soldé et l'impôt de plus-value payé — vient alimenter le plan
        à la date prévue, et les loyers correspondants disparaissent de la projection.
      </p>
      {q.ventes.length === 0 && <p className="vide">Aucune vente prévue.</p>}
      {q.ventes.map((v, i) => (
        <fieldset key={i} className="carte-saisie">
          <legend>
            {v.libelle || `Vente ${i + 1}`}
            <button type="button" className="lien danger" onClick={() => set('ventes', q.ventes.filter((_, j) => j !== i))}>
              Retirer
            </button>
          </legend>
          <div className="grille">
            <TextField label="Libellé" value={v.libelle} onChange={(x) => majVente(i, { libelle: x })} />
            <SelectField
              label="Bien vendu"
              value={cleBien(v.bien)}
              options={biens}
              onChange={(x) => majVente(i, { bien: x === 'direct' ? { type: 'locatif_direct' } : { type: 'sci', id: x.slice(4) } })}
            />
            <NumberField label="Dans" unite="ans" value={v.dans_ans} onChange={(x) => majVente(i, { dans_ans: x })} min={0} max={40} entier />
            <NumberField
              label="Part vendue"
              unite="%"
              value={v.part_vendue_pct}
              onChange={(x) => majVente(i, { part_vendue_pct: x })}
              min={0}
              max={100}
            />
            <NumberField
              label="Prix de vente attendu"
              unite="€"
              value={v.prix_vente}
              onChange={(x) => majVente(i, { prix_vente: x })}
              min={0}
              aide="En euros d'aujourd'hui : l'outil applique la revalorisation des hypothèses."
            />
            <NumberField
              label="Prix d'acquisition"
              unite="€"
              value={v.prix_acquisition}
              onChange={(x) => majVente(i, { prix_acquisition: x })}
              min={0}
              aide="Frais de notaire et travaux compris : c'est la base de la plus-value."
            />
            <NumberField
              label="Détenu depuis"
              unite="ans"
              value={v.detention_ans}
              onChange={(x) => majVente(i, { detention_ans: x })}
              min={0}
              max={80}
              entier
              aide="Hors SCI à l'IS : exonération d'impôt à 22 ans et de prélèvements sociaux à 30 ans."
            />
            <NumberField
              label="Frais de vente"
              unite="%"
              value={v.frais_vente_pct}
              onChange={(x) => majVente(i, { frais_vente_pct: x })}
              min={0}
              max={20}
              aide="Agence et diagnostics."
            />
          </div>
        </fieldset>
      ))}
      <button
        type="button"
        className="bouton secondaire"
        onClick={() =>
          set('ventes', [
            ...q.ventes,
            {
              libelle: `Vente ${q.ventes.length + 1}`,
              bien: { type: 'locatif_direct' },
              dans_ans: 5,
              part_vendue_pct: 100,
              prix_vente: 0,
              prix_acquisition: 0,
              detention_ans: 0,
              frais_vente_pct: 5,
            },
          ])
        }
      >
        + Programmer une vente
      </button>
    </>
  )
}
