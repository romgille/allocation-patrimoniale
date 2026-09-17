// Module produit par wasm-bindgen (`make wasm`, dossier web/wasm-pkg, non versionné).
// Déclaré ici pour que la vérification des types fonctionne sans l'avoir construit.
declare module 'allocation-wasm' {
  export default function init(): Promise<unknown>
  export function hypotheses(): string
  export function exemple(): string
  export function calculer(requete: string): string
}
