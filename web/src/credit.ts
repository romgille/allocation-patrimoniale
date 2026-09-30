/**
 * Mensualité théorique d'un prêt amortissable, reprise du moteur Rust (`crates/credit.rs`)
 * pour donner un retour immédiat pendant la saisie. Le moteur reste la référence : c'est
 * lui qui décide si la saisie est cohérente.
 */
export function mensualiteTheorique(restantDu: number, taux: number, mois: number): number {
  if (mois <= 0 || restantDu <= 0) return 0
  const i = taux / 12
  if (Math.abs(i) < 1e-12) return restantDu / mois
  const f = (1 + i) ** mois
  return (restantDu * i * f) / (f - 1)
}
