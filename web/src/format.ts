const eurFmt = new Intl.NumberFormat('fr-FR', { style: 'currency', currency: 'EUR', maximumFractionDigits: 0 })
const eurCompact = new Intl.NumberFormat('fr-FR', {
  style: 'currency',
  currency: 'EUR',
  notation: 'compact',
  maximumFractionDigits: 1,
})
const numFmt = new Intl.NumberFormat('fr-FR', { maximumFractionDigits: 2 })

export const eur = (v: number): string => eurFmt.format(Math.round(v) || 0)
export const eurK = (v: number): string => eurCompact.format(v)
export const num = (v: number): string => numFmt.format(v)
/** Fraction → « 12,5 % ». */
export const pct = (v: number, d = 1): string =>
  `${(v * 100 || 0).toLocaleString('fr-FR', { minimumFractionDigits: 0, maximumFractionDigits: d })} %`
/** Points de pourcentage signés. */
export const pts = (v: number): string =>
  `${v > 0 ? '+' : v < 0 ? '−' : ''}${Math.abs(v).toLocaleString('fr-FR', { maximumFractionDigits: 1 })} pt`

/** Lecture tolérante d'un nombre saisi en français. */
export function parseNombre(s: string): number | null {
  const t = s.replace(/\s| |€|%/g, '').replace(',', '.')
  if (t === '' || t === '-' || t === '.') return null
  const n = Number(t)
  return Number.isFinite(n) ? n : null
}
