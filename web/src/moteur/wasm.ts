import init, * as wasm from 'allocation-wasm'
import type { Hypotheses } from '../bindings/Hypotheses'
import type { Questionnaire } from '../bindings/Questionnaire'
import type { Moteur, ReponseCalcul } from './contrat'

// Le module WebAssembly est chargé une seule fois, au premier appel.
let pret: Promise<unknown> | undefined
const charger = (): Promise<unknown> => (pret ??= init())

async function calculer(
  questionnaire: Questionnaire,
  hypotheses: Hypotheses,
  signal?: AbortSignal,
): Promise<ReponseCalcul> {
  await charger()
  signal?.throwIfAborted()
  return JSON.parse(wasm.calculer(JSON.stringify({ questionnaire, hypotheses }))) as ReponseCalcul
}

export default {
  chargerHypotheses: async () => {
    await charger()
    return JSON.parse(wasm.hypotheses()) as Hypotheses
  },
  chargerExemple: async () => {
    await charger()
    return JSON.parse(wasm.exemple()) as Questionnaire
  },
  calculer,
  mentionDonnees: 'Les calculs se font dans votre navigateur : aucune donnée saisie ne quitte votre appareil.',
} satisfies Moteur
