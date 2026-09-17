import type { ErreursValidation } from './bindings/ErreursValidation'
import type { Hypotheses } from './bindings/Hypotheses'
import type { Questionnaire } from './bindings/Questionnaire'
import type { Resultat } from './bindings/Resultat'

export type ReponseCalcul =
  | { ok: true; resultat: Resultat }
  | { ok: false; erreurs: string[] }

async function getJson<T>(url: string): Promise<T> {
  const r = await fetch(url)
  if (!r.ok) throw new Error(`${url} : HTTP ${r.status}`)
  return (await r.json()) as T
}

export const chargerHypotheses = (): Promise<Hypotheses> => getJson<Hypotheses>('/api/hypotheses')
export const chargerExemple = (): Promise<Questionnaire> => getJson<Questionnaire>('/api/exemple')

export async function calculer(
  questionnaire: Questionnaire,
  hypotheses: Hypotheses,
  signal?: AbortSignal,
): Promise<ReponseCalcul> {
  const r = await fetch('/api/calcul', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ questionnaire, hypotheses }),
    signal,
  })
  if (r.status === 422) {
    const e = (await r.json()) as ErreursValidation
    return { ok: false, erreurs: e.erreurs }
  }
  if (!r.ok) throw new Error(`Calcul : HTTP ${r.status}`)
  return { ok: true, resultat: (await r.json()) as Resultat }
}
