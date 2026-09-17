import { useEffect, useRef, useState } from 'react'
import { calculer, chargerExemple, chargerHypotheses, mentionDonnees } from './moteur'
import type { Hypotheses } from './bindings/Hypotheses'
import type { Questionnaire } from './bindings/Questionnaire'
import type { Resultat } from './bindings/Resultat'
import { nouvelAdulte } from './components/FoyerForm'
import { HypothesesForm } from './components/HypothesesForm'
import { ETAPES, QuestionnaireForm, type Etape } from './components/QuestionnaireForm'
import { Resultats } from './components/Resultats'
import { eur } from './format'
import { creerSauvegarde, lireSauvegarde, type Sauvegarde } from './migration'

type Vue = 'saisie' | 'resultats' | 'hypotheses'
const CLE_BROUILLON = 'allocation-patrimoniale:brouillon'

function questionnaireVierge(): Questionnaire {
  return {
    adultes: [nouvelAdulte(1)],
    tmi: '30',
    rp: 'locataire',
    lep_eligible: false,
    depenses: [],
    epargne_mensuelle_forcee: null,
    h_fire: 15,
    r_cible: 1500,
    taux_retrait_pct: 3.5,
    autres_revenus_passifs: 0,
    enfants: [],
    projets: [],
    avoirs: {
      livrets: 0,
      liquidites_a_investir: 0,
      etf_monde: 0,
      fonds_euros_obligations: 0,
      scpi: 0,
      or: 0,
      actions_directes: 0,
      crowdfunding_immo: 0,
      crowdfunding_enr: 0,
      crypto: 0,
      private_equity: 0,
    },
    v_immo_loc: 0,
    cf_immo: 0,
    dettes: [],
    contraintes: { esg: false, refus_crypto: false, refus_immo_direct: false, crypto_souhaitee_pct: 0 },
  }
}

function lireBrouillon(): Sauvegarde | null {
  try {
    const s = localStorage.getItem(CLE_BROUILLON)
    if (!s) return null
    return lireSauvegarde(JSON.parse(s))?.sauvegarde ?? null
  } catch {
    return null
  }
}

export default function App() {
  const [q, setQ] = useState<Questionnaire | null>(null)
  const [h, setH] = useState<Hypotheses | null>(null)
  const [defauts, setDefauts] = useState<Hypotheses | null>(null)
  const [resultat, setResultat] = useState<Resultat | null>(null)
  const [erreurs, setErreurs] = useState<string[]>([])
  const [erreurReseau, setErreurReseau] = useState<string | null>(null)
  const [calculEnCours, setCalculEnCours] = useState(false)
  const [vue, setVue] = useState<Vue>('saisie')
  const [etape, setEtape] = useState<Etape>('foyer')
  const [info, setInfo] = useState<string | null>(null)
  const fichier = useRef<HTMLInputElement>(null)

  // Chargement initial : brouillon local, sinon exemple.
  useEffect(() => {
    let annule = false
    ;(async () => {
      try {
        const d = await chargerHypotheses()
        if (annule) return
        setDefauts(d)
        const b = lireBrouillon()
        if (b) {
          setQ(b.questionnaire)
          setH({ ...d, ...b.hypotheses })
        } else {
          setQ(await chargerExemple())
          setH(d)
        }
      } catch (e) {
        setErreurReseau(`Impossible de charger le moteur de calcul : ${String(e)}`)
      }
    })()
    return () => {
      annule = true
    }
  }, [])

  // Recalcul automatique (anti-rebond) + brouillon local.
  useEffect(() => {
    if (!q || !h) return
    try {
      localStorage.setItem(CLE_BROUILLON, JSON.stringify(creerSauvegarde(q, h)))
    } catch {
      /* stockage indisponible : sans conséquence */
    }
    const ctrl = new AbortController()
    const t = window.setTimeout(async () => {
      setCalculEnCours(true)
      try {
        const r = await calculer(q, h, ctrl.signal)
        if (r.ok) {
          setResultat(r.resultat)
          setErreurs([])
        } else {
          setErreurs(r.erreurs)
        }
        setErreurReseau(null)
      } catch (e) {
        if (!ctrl.signal.aborted) setErreurReseau(`Calcul impossible : ${String(e)}`)
      } finally {
        if (!ctrl.signal.aborted) setCalculEnCours(false)
      }
    }, 250)
    return () => {
      ctrl.abort()
      window.clearTimeout(t)
    }
  }, [q, h])

  const exporter = () => {
    if (!q || !h) return
    const blob = new Blob([JSON.stringify(creerSauvegarde(q, h), null, 2)], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `allocation-${new Date().toISOString().slice(0, 10)}.json`
    a.click()
    URL.revokeObjectURL(url)
  }

  const importer = async (f: File) => {
    try {
      const lu = lireSauvegarde(JSON.parse(await f.text()))
      if (!lu) throw new Error('format inattendu')
      const x = lu.sauvegarde
      setQ(x.questionnaire)
      setH(defauts ? { ...defauts, ...x.hypotheses } : x.hypotheses)
      setErreurReseau(null)
      setInfo(
        lu.migree
          ? "Fichier de l'ancienne version importé : complétez les adultes, le budget par poste et les projets."
          : `Fichier du ${new Date(x.date).toLocaleDateString('fr-FR')} importé.`,
      )
    } catch (e) {
      setErreurReseau(`Import impossible : ${String(e)}`)
    }
  }

  const iEtape = ETAPES.findIndex((e) => e.id === etape)
  const nbCritiques = resultat?.alertes.filter((a) => a.gravite === 'critique').length ?? 0

  return (
    <div className="app">
      <header className="entete">
        <div className="titre">
          <h1>Allocation patrimoniale</h1>
          <p>Budget, projets, études des enfants et revenus passifs du foyer · résident fiscal français</p>
        </div>
        <div className="actions">
          <button type="button" className="bouton discret" onClick={() => void chargerExemple().then(setQ)}>
            Exemple
          </button>
          <button
            type="button"
            className="bouton discret"
            onClick={() => {
              setQ(questionnaireVierge())
              setEtape('foyer')
              setVue('saisie')
            }}
          >
            Vierge
          </button>
          <button type="button" className="bouton discret" onClick={() => fichier.current?.click()}>
            Importer
          </button>
          <button type="button" className="bouton secondaire" onClick={exporter} disabled={!q}>
            Exporter
          </button>
          <input
            ref={fichier}
            type="file"
            accept="application/json,.json"
            hidden
            onChange={(e) => {
              const f = e.target.files?.[0]
              if (f) void importer(f)
              e.target.value = ''
            }}
          />
        </div>
      </header>

      <nav className="onglets" role="tablist">
        {(
          [
            ['saisie', 'Questionnaire'],
            ['resultats', 'Résultats'],
            ['hypotheses', 'Hypothèses'],
          ] as const
        ).map(([id, label]) => (
          <button key={id} role="tab" aria-selected={vue === id} className={vue === id ? 'actif' : ''} onClick={() => setVue(id)}>
            {label}
            {id === 'resultats' && nbCritiques > 0 && <span className="pastille-compteur">{nbCritiques}</span>}
          </button>
        ))}
        <span className={`etat ${calculEnCours ? 'actif' : ''}`} aria-live="polite">
          {calculEnCours ? 'Calcul…' : resultat ? 'À jour' : ''}
        </span>
      </nav>

      {erreurReseau && <div className="bandeau erreur">{erreurReseau}</div>}
      {info && (
        <div className="bandeau info">
          {info}
          <button type="button" className="lien" onClick={() => setInfo(null)}>
            Fermer
          </button>
        </div>
      )}
      {erreurs.length > 0 && (
        <div className="bandeau erreur">
          <strong>Saisie à corriger :</strong>
          <ul>
            {erreurs.map((e) => (
              <li key={e}>{e}</li>
            ))}
          </ul>
        </div>
      )}

      {!q || !h ? (
        !erreurReseau && <p className="chargement">Chargement…</p>
      ) : (
        <main>
          {vue === 'saisie' && (
            <div className="saisie-layout">
              <ol className="etapes-nav">
                {ETAPES.map((e, i) => (
                  <li key={e.id}>
                    <button type="button" className={e.id === etape ? 'actif' : ''} onClick={() => setEtape(e.id)}>
                      <span className="num-etape">{i + 1}</span>
                      {e.titre}
                    </button>
                  </li>
                ))}
              </ol>
              <div className="formulaire">
                <h2>{ETAPES[iEtape]?.titre}</h2>
                <QuestionnaireForm etape={etape} q={q} onChange={setQ} />
                <div className="navigation">
                  <button
                    type="button"
                    className="bouton secondaire"
                    disabled={iEtape === 0}
                    onClick={() => {
                      const p = ETAPES[iEtape - 1]
                      if (p) setEtape(p.id)
                    }}
                  >
                    ← Précédent
                  </button>
                  {iEtape < ETAPES.length - 1 ? (
                    <button
                      type="button"
                      className="bouton"
                      onClick={() => {
                        const n = ETAPES[iEtape + 1]
                        if (n) setEtape(n.id)
                      }}
                    >
                      Suivant →
                    </button>
                  ) : (
                    <button type="button" className="bouton" onClick={() => setVue('resultats')}>
                      Voir les résultats →
                    </button>
                  )}
                </div>
              </div>
              {resultat && (
                <aside className="apercu" aria-label="Aperçu">
                  <h3>Aperçu en direct</h3>
                  <dl className="liste-def">
                    <dt>Revenus</dt>
                    <dd>{eur(resultat.budget.total_revenus_eur)}</dd>
                    <dt>Épargne</dt>
                    <dd className={resultat.budget.capacite_calculee_eur < 0 ? 'ko' : ''}>
                      {eur(resultat.budget.epargne_retenue_eur)}/mois
                    </dd>
                    <dt>Profil</dt>
                    <dd>{resultat.poche_2.profil_libelle}</dd>
                    <dt>Précaution</dt>
                    <dd>
                      {resultat.poche_0.manque_eur > 0 ? `manque ${eur(resultat.poche_0.manque_eur)}` : 'constituée'}
                    </dd>
                    <dt>Projets</dt>
                    <dd>{eur(resultat.poche_1.reduce((s, x) => s + x.epargne_mensuelle_eur, 0))}/mois</dd>
                    <dt>Capital cible</dt>
                    <dd>{eur(resultat.poche_2.capital_cible_eur)}</dd>
                    <dt>Objectif</dt>
                    <dd className={resultat.poche_2.atteignable ? 'ok' : 'ko'}>
                      {resultat.poche_2.atteignable ? '✓ atteignable' : '✗ non atteignable'}
                    </dd>
                  </dl>
                  <button type="button" className="lien" onClick={() => setVue('resultats')}>
                    {resultat.alertes.length} alerte{resultat.alertes.length > 1 ? 's' : ''} · détails →
                  </button>
                </aside>
              )}
            </div>
          )}
          {vue === 'resultats' &&
            (resultat ? <Resultats r={resultat} /> : <p className="chargement">Calcul en cours…</p>)}
          {vue === 'hypotheses' && <HypothesesForm h={h} defauts={defauts} onChange={setH} />}
        </main>
      )}

      <footer className="pied">
        Outil informatif et pédagogique : ne constitue pas un conseil en investissement. Faire valider la fiscalité
        (PFU, prélèvements sociaux, LMNP, PER) par une source à jour. {mentionDonnees}
      </footer>
    </div>
  )
}
