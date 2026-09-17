import type { Hypotheses } from '../bindings/Hypotheses'
import type { Questionnaire } from '../bindings/Questionnaire'
import type { Resultat } from '../bindings/Resultat'

export type ReponseCalcul =
  | { ok: true; resultat: Resultat }
  | { ok: false; erreurs: string[] }

/**
 * Contrat commun aux deux moteurs :
 * - `http` : l'API Rust (version Docker) ;
 * - `wasm` : le moteur Rust compilé en WebAssembly, exécuté dans le navigateur (GitHub Pages).
 * Le moteur est choisi à la construction (voir vite.config.ts).
 */
export interface Moteur {
  chargerHypotheses(): Promise<Hypotheses>
  chargerExemple(): Promise<Questionnaire>
  calculer(questionnaire: Questionnaire, hypotheses: Hypotheses, signal?: AbortSignal): Promise<ReponseCalcul>
  /** Phrase du pied de page sur le devenir des données saisies. */
  mentionDonnees: string
}
