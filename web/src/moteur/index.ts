// `@moteur` pointe vers http.ts (Docker, développement) ou wasm.ts (GitHub Pages) : voir vite.config.ts.
import moteur from '@moteur'

export type { ReponseCalcul } from './contrat'
export const { chargerHypotheses, chargerExemple, calculer, mentionDonnees } = moteur
