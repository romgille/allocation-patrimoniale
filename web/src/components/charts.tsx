import { useEffect, useRef, useState, type ReactNode, type RefObject } from 'react'
import type { LigneAllocation } from '../bindings/LigneAllocation'
import type { PointProjection } from '../bindings/PointProjection'
import { eur, eurK, pct, pts } from '../format'

function useLargeur<T extends HTMLElement>(): [RefObject<T | null>, number] {
  const ref = useRef<T>(null)
  const [l, setL] = useState(600)
  useEffect(() => {
    const el = ref.current
    if (!el) return
    const ro = new ResizeObserver(([e]) => {
      if (e) setL(Math.max(260, Math.floor(e.contentRect.width)))
    })
    ro.observe(el)
    return () => ro.disconnect()
  }, [])
  return [ref, l]
}

function Infobulle({ x, y, children }: { x: number; y: number; children: ReactNode }) {
  return (
    <div className="infobulle" style={{ left: x, top: y }} role="status">
      {children}
    </div>
  )
}

/* ------------------------------------------------------------------ */
/* Allocation actuelle vs cible (barre + repère de cible)              */
/* ------------------------------------------------------------------ */

export function AllocationChart({ lignes }: { lignes: LigneAllocation[] }) {
  const [survol, setSurvol] = useState<number | null>(null)
  const max = Math.max(0.1, ...lignes.flatMap((l) => [l.cible_pct, l.actuelle_pct]))
  const echelle = Math.min(1, Math.ceil(max * 10) / 10)
  const graduations = Array.from({ length: Math.round(echelle * 10) + 1 }, (_, i) => i / 10).filter(
    (_, i, a) => a.length <= 6 || i % 2 === 0,
  )
  return (
    <figure className="graphe">
      <div className="legende" aria-hidden="true">
        <span>
          <i className="pastille serie-1" /> Actuelle
        </span>
        <span>
          <i className="repere" /> Cible
        </span>
      </div>
      <div className="barres" role="img" aria-label="Allocation actuelle comparée à la cible, par ligne">
        {lignes.map((l, i) => (
          <div
            key={l.ligne}
            className={`barre-ligne ${survol === i ? 'survol' : ''}`}
            onMouseEnter={() => setSurvol(i)}
            onMouseLeave={() => setSurvol(null)}
            onFocus={() => setSurvol(i)}
            onBlur={() => setSurvol(null)}
            tabIndex={0}
          >
            <span className="barre-libelle">{l.libelle}</span>
            <div className="piste-barre">
              {graduations.map((g) => (
                <span key={g} className="grille-v" style={{ left: `${(g / echelle) * 100}%` }} />
              ))}
              <span className="remplissage" style={{ width: `${(l.actuelle_pct / echelle) * 100}%` }} />
              <span className="cible" style={{ left: `${(l.cible_pct / echelle) * 100}%` }} />
            </div>
            <span className="barre-valeur">
              {pct(l.actuelle_pct, 0)} <span className="muet">/ {pct(l.cible_pct, 0)}</span>
            </span>
            {survol === i && (
              <div className="infobulle infobulle-barre">
                <strong>{l.libelle}</strong>
                <span>
                  Actuelle : {pct(l.actuelle_pct)} ({eur(l.actuel_eur)})
                </span>
                <span>
                  Cible : {pct(l.cible_pct)} ({eur(l.cible_eur)})
                </span>
                <span>Écart : {pts(l.ecart_points)}</span>
              </div>
            )}
          </div>
        ))}
        <div className="barre-ligne axe" aria-hidden="true">
          <span />
          <div className="piste-barre">
            {graduations.map((g) => (
              <span key={g} className="graduation" style={{ left: `${(g / echelle) * 100}%` }}>
                {pct(g, 0)}
              </span>
            ))}
          </div>
          <span />
        </div>
      </div>
    </figure>
  )
}

/* ------------------------------------------------------------------ */
/* Projection du capital de la poche 2                                  */
/* ------------------------------------------------------------------ */

export function ProjectionChart({ points }: { points: PointProjection[] }) {
  const [ref, largeur] = useLargeur<HTMLDivElement>()
  const [survol, setSurvol] = useState<number | null>(null)
  const H = 260
  const m = { g: 56, d: 16, h: 16, b: 28 }
  const w = largeur - m.g - m.d
  const h = H - m.h - m.b
  if (points.length === 0) return null
  const nMax = points[points.length - 1]?.annee ?? 1
  const vMax =
    Math.max(1, ...points.flatMap((p) => [p.capital_flux_disponible, p.capital_epargne_necessaire, p.cible])) * 1.08
  const pas = Math.pow(10, Math.floor(Math.log10(vMax)))
  const tick = vMax / pas > 5 ? pas * 2 : vMax / pas > 2 ? pas : pas / 2
  const yTicks = Array.from({ length: Math.floor(vMax / tick) + 1 }, (_, i) => i * tick)
  const x = (a: number) => m.g + (nMax === 0 ? 0 : (a / nMax) * w)
  const y = (v: number) => m.h + h - (v / vMax) * h
  const chemin = (f: (p: PointProjection) => number) =>
    points.map((p, i) => `${i === 0 ? 'M' : 'L'}${x(p.annee).toFixed(1)},${y(f(p)).toFixed(1)}`).join('')
  const cible = points[0]?.cible ?? 0
  const pasX = nMax > 20 ? 5 : nMax > 10 ? 2 : 1
  const p = survol !== null ? points[survol] : undefined

  return (
    <figure className="graphe">
      <div className="legende" aria-hidden="true">
        <span>
          <i className="trait serie-1" /> Avec l'épargne du foyer
        </span>
        <span>
          <i className="trait serie-2" /> Versement constant nécessaire
        </span>
        <span>
          <i className="trait pointille" /> Capital cible
        </span>
      </div>
      <div ref={ref} className="svg-conteneur">
        <svg
          width={largeur}
          height={H}
          role="img"
          aria-label="Projection du capital de la poche 2 jusqu'à l'échéance"
          onMouseLeave={() => setSurvol(null)}
          onMouseMove={(e) => {
            const r = e.currentTarget.getBoundingClientRect()
            const a = Math.round(((e.clientX - r.left - m.g) / w) * nMax)
            setSurvol(Math.max(0, Math.min(points.length - 1, a)))
          }}
        >
          {yTicks.map((t) => (
            <g key={t}>
              <line className="grille" x1={m.g} x2={m.g + w} y1={y(t)} y2={y(t)} />
              <text className="axe-texte" x={m.g - 8} y={y(t)} textAnchor="end" dominantBaseline="middle">
                {eurK(t)}
              </text>
            </g>
          ))}
          <line className="ligne-base" x1={m.g} x2={m.g + w} y1={y(0)} y2={y(0)} />
          {points
            .filter((pt) => pt.annee % pasX === 0)
            .map((pt) => (
              <text key={pt.annee} className="axe-texte" x={x(pt.annee)} y={H - 8} textAnchor="middle">
                {pt.annee === 0 ? 'auj.' : `${pt.annee} a`}
              </text>
            ))}
          {cible > 0 && <line className="reference" x1={m.g} x2={m.g + w} y1={y(cible)} y2={y(cible)} />}
          <path className="courbe serie-2" d={chemin((pt) => pt.capital_epargne_necessaire)} />
          <path className="courbe serie-1" d={chemin((pt) => pt.capital_flux_disponible)} />
          {p && (
            <g>
              <line className="curseur" x1={x(p.annee)} x2={x(p.annee)} y1={m.h} y2={m.h + h} />
              <circle className="point serie-2" cx={x(p.annee)} cy={y(p.capital_epargne_necessaire)} r={4} />
              <circle className="point serie-1" cx={x(p.annee)} cy={y(p.capital_flux_disponible)} r={4} />
            </g>
          )}
        </svg>
        {p && (
          <Infobulle x={Math.min(x(p.annee) + 12, largeur - 230)} y={8}>
            <strong>{p.annee === 0 ? "Aujourd'hui" : `Dans ${p.annee} an${p.annee > 1 ? 's' : ''}`}</strong>
            <span>
              <i className="trait serie-1" /> Épargne du foyer : {eur(p.capital_flux_disponible)}
            </span>
            <span>
              <i className="trait serie-2" /> Versement constant : {eur(p.capital_epargne_necessaire)}
            </span>
            <span>
              <i className="trait pointille" /> Cible : {eur(p.cible)}
            </span>
          </Infobulle>
        )}
      </div>
    </figure>
  )
}

/* ------------------------------------------------------------------ */
/* Répartition du flux mensuel                                          */
/* ------------------------------------------------------------------ */

export type SegmentFlux = { cle: string; libelle: string; valeur: number; serie: 1 | 2 | 3 | 4 | 'neutre' }

export function FluxBar({ segments, total }: { segments: SegmentFlux[]; total: number }) {
  const [survol, setSurvol] = useState<string | null>(null)
  const visibles = segments.filter((s) => s.valeur > 0.5)
  const t = Math.max(total, 1)
  return (
    <figure className="graphe">
      <div className="empile" role="img" aria-label="Répartition de l'épargne mensuelle">
        {visibles.map((s) => (
          <span
            key={s.cle}
            className={`segment serie-${s.serie} ${survol && survol !== s.cle ? 'estompe' : ''}`}
            style={{ flexGrow: s.valeur / t }}
            onMouseEnter={() => setSurvol(s.cle)}
            onMouseLeave={() => setSurvol(null)}
            title={`${s.libelle} : ${eur(s.valeur)}`}
          />
        ))}
      </div>
      <ul className="legende-valeurs">
        {segments.map((s) => (
          <li
            key={s.cle}
            className={survol && survol !== s.cle ? 'estompe' : ''}
            onMouseEnter={() => setSurvol(s.cle)}
            onMouseLeave={() => setSurvol(null)}
          >
            <i className={`pastille serie-${s.serie}`} />
            <span>{s.libelle}</span>
            <strong>{eur(s.valeur)}</strong>
            <span className="muet">{pct(s.valeur / t, 0)}</span>
          </li>
        ))}
      </ul>
    </figure>
  )
}
