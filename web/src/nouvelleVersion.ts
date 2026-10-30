import { useEffect, useState } from 'react'

const INTERVALLE_MS = 5 * 60 * 1000

// Détecte un nouveau déploiement GitHub Pages en comparant la version embarquée à /version.json.
// Inactif hors mode pages (version vide).
export function useNouvelleVersion(): boolean {
  const [disponible, setDisponible] = useState(false)

  useEffect(() => {
    if (!__VERSION_APP__ || disponible) return
    const verifier = async () => {
      try {
        const r = await fetch(`${import.meta.env.BASE_URL}version.json?t=${Date.now()}`, { cache: 'no-store' })
        if (!r.ok) return
        const { version } = (await r.json()) as { version?: string }
        if (version && version !== __VERSION_APP__) setDisponible(true)
      } catch {
        // hors ligne : on réessaiera
      }
    }
    const surVisibilite = () => {
      if (document.visibilityState === 'visible') void verifier()
    }
    const id = window.setInterval(verifier, INTERVALLE_MS)
    document.addEventListener('visibilitychange', surVisibilite)
    return () => {
      window.clearInterval(id)
      document.removeEventListener('visibilitychange', surVisibilite)
    }
  }, [disponible])

  return disponible
}
