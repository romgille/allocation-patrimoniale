import type { Allocation } from './bindings/Allocation'
import type { Ligne } from './bindings/Ligne'

export const LIGNES: { cle: keyof Allocation & Ligne; libelle: string }[] = [
  { cle: 'etf_monde', libelle: 'ETF actions monde' },
  { cle: 'fonds_euros_obligations', libelle: 'Fonds euros / obligations' },
  { cle: 'immobilier', libelle: 'Immobilier (locatif net + SCPI)' },
  { cle: 'or', libelle: 'Or' },
  { cle: 'actions_directes', libelle: 'Actions en direct' },
  { cle: 'crowdfunding', libelle: 'Crowdfunding (immo + ENR)' },
  { cle: 'crypto', libelle: 'Crypto' },
  { cle: 'private_equity', libelle: 'Private equity' },
]
