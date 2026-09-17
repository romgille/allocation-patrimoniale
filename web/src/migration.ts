import type { Hypotheses } from './bindings/Hypotheses'
import type { Questionnaire } from './bindings/Questionnaire'
import type { StatutRp } from './bindings/StatutRp'
import type { TempsGestion } from './bindings/TempsGestion'
import type { Tmi } from './bindings/Tmi'

export const VERSION_SAUVEGARDE = 2

/** Fichier de sauvegarde local (export / import / brouillon). */
export type Sauvegarde = {
  format: 'allocation-patrimoniale'
  version: typeof VERSION_SAUVEGARDE
  date: string
  questionnaire: Questionnaire
  hypotheses: Hypotheses
}

/** Questionnaire de la version 1 (un seul profil pour tout le foyer). */
type QuestionnaireV1 = Omit<Questionnaire, 'adultes' | 'depenses' | 'epargne_mensuelle_forcee' | 'projets'> & {
  age: number
  en_couple: boolean
  revenus_nets_mensuels: number
  depenses_mensuelles: number
  tmi: Tmi
  rp: StatutRp
  stab_revenus: number
  tol_risque: number
  temps_gestion: TempsGestion
  abondement_employeur: boolean
  e_mois: number
}

function estObjet(x: unknown): x is Record<string, unknown> {
  return typeof x === 'object' && x !== null
}

function versV2(v1: QuestionnaireV1): Questionnaire {
  const { age, en_couple, revenus_nets_mensuels, depenses_mensuelles, stab_revenus, tol_risque, temps_gestion, abondement_employeur, e_mois, ...reste } = v1
  const profil = { age, stab_revenus, tol_risque, temps_gestion }
  return {
    ...reste,
    adultes: [
      { prenom: 'Adulte 1', revenus: [{ libelle: 'Revenus du foyer', montant_mensuel: revenus_nets_mensuels }], abondement_employeur, ...profil },
      ...(en_couple ? [{ prenom: 'Adulte 2', revenus: [], abondement_employeur: false, ...profil }] : []),
    ],
    // Les anciennes dépenses incluaient souvent les crédits : on les garde telles quelles,
    // et on impose l'ancienne capacité d'épargne pour ne pas changer les résultats.
    depenses: [{ poste: 'autre', libelle: 'Dépenses du foyer (import)', montant_mensuel: depenses_mensuelles }],
    epargne_mensuelle_forcee: e_mois,
    projets: [],
  }
}

/**
 * Lit un fichier de sauvegarde, en migrant les anciennes versions.
 * Renvoie `null` si le contenu n'est pas reconnu.
 */
export function lireSauvegarde(x: unknown): { sauvegarde: Sauvegarde; migree: boolean } | null {
  if (!estObjet(x) || x.format !== 'allocation-patrimoniale') return null
  if (!estObjet(x.questionnaire) || !estObjet(x.hypotheses)) return null
  const hypotheses = x.hypotheses as Hypotheses
  const date = typeof x.date === 'string' ? x.date : new Date().toISOString()
  if (x.version === VERSION_SAUVEGARDE && Array.isArray(x.questionnaire.adultes)) {
    return {
      sauvegarde: { format: 'allocation-patrimoniale', version: VERSION_SAUVEGARDE, date, questionnaire: x.questionnaire as Questionnaire, hypotheses },
      migree: false,
    }
  }
  if (x.version === 1 && typeof x.questionnaire.age === 'number') {
    return {
      sauvegarde: {
        format: 'allocation-patrimoniale',
        version: VERSION_SAUVEGARDE,
        date,
        questionnaire: versV2(x.questionnaire as unknown as QuestionnaireV1),
        hypotheses,
      },
      migree: true,
    }
  }
  return null
}

export function creerSauvegarde(q: Questionnaire, h: Hypotheses): Sauvegarde {
  return { format: 'allocation-patrimoniale', version: VERSION_SAUVEGARDE, date: new Date().toISOString(), questionnaire: q, hypotheses: h }
}
