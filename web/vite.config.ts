import { fileURLToPath } from 'node:url'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

const chemin = (p: string): string => fileURLToPath(new URL(p, import.meta.url))

// Deux façons de construire le front :
// - par défaut (Docker, `npm run dev`) : le calcul passe par l'API Rust sur /api ;
// - `--mode pages` (GitHub Pages) : le moteur Rust compilé en WebAssembly tourne dans le navigateur.
//   Nécessite `make wasm` au préalable. Chemin de publication réglable avec BASE_PATH.
export default defineConfig(({ mode }) => {
  const pages = mode === 'pages'
  const version = process.env.GITHUB_SHA ?? String(Date.now())
  return {
    define: { __VERSION_APP__: JSON.stringify(pages ? version : '') },
    base: pages ? (process.env.BASE_PATH ?? '/allocation-patrimoniale/') : '/',
    plugins: [
      react(),
      // Publie la version du déploiement pour que l'appli détecte qu'une mise à jour est disponible.
      {
        name: 'version-deploiement',
        generateBundle() {
          if (pages) this.emitFile({ type: 'asset', fileName: 'version.json', source: JSON.stringify({ version }) })
        },
      },
    ],
    resolve: {
      alias: {
        '@moteur': chemin(pages ? './src/moteur/wasm.ts' : './src/moteur/http.ts'),
        ...(pages ? { 'allocation-wasm': chemin('./wasm-pkg/allocation_wasm.js') } : {}),
      },
    },
    server: {
      // En développement, /api est relayé vers l'API Rust (cargo run -p api).
      proxy: pages ? undefined : { '/api': process.env.API_URL ?? 'http://localhost:8080' },
    },
  }
})
