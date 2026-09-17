import { useEffect, useId, useState, type ReactNode } from 'react'
import { parseNombre } from '../format'

type BaseProps = { label: string; aide?: ReactNode }

function Champ({ label, aide, id, children }: BaseProps & { id: string; children: ReactNode }) {
  return (
    <div className="champ">
      <label htmlFor={id}>{label}</label>
      {children}
      {aide && <p className="aide">{aide}</p>}
    </div>
  )
}

type NumberProps = BaseProps & {
  value: number
  onChange: (v: number) => void
  unite?: string
  min?: number
  max?: number
  step?: number
  entier?: boolean
  /** Facteur d'affichage (100 pour saisir un % stocké en fraction). */
  facteur?: number
}

export function NumberField({ label, aide, value, onChange, unite, min, max, step, entier, facteur = 1 }: NumberProps) {
  const id = useId()
  const affiche = (v: number) => {
    const x = v * facteur
    return String(Math.round(x * 1e6) / 1e6).replace('.', ',')
  }
  const [texte, setTexte] = useState(affiche(value))
  const [focus, setFocus] = useState(false)
  useEffect(() => {
    // Resynchronise l'affichage quand la valeur change de l'extérieur (import, exemple…).
    if (!focus) setTexte(affiche(value))
  }, [value, focus])

  const valider = (s: string) => {
    let n = parseNombre(s)
    if (n === null) return
    if (entier) n = Math.round(n)
    if (min !== undefined && n < min) return
    if (max !== undefined && n > max) return
    onChange(n / facteur)
  }
  const n = parseNombre(texte)
  const invalide =
    n === null || (min !== undefined && n < min) || (max !== undefined && n > max) || (entier === true && !Number.isInteger(n))

  return (
    <Champ label={label} aide={aide} id={id}>
      <div className={`saisie ${invalide ? 'invalide' : ''}`}>
        <input
          id={id}
          inputMode="decimal"
          value={texte}
          step={step}
          aria-invalid={invalide}
          onFocus={() => setFocus(true)}
          onBlur={() => setFocus(false)}
          onChange={(e) => {
            setTexte(e.target.value)
            valider(e.target.value)
          }}
        />
        {unite && <span className="unite">{unite}</span>}
      </div>
    </Champ>
  )
}

type Option<T extends string> = { value: T; label: string }

export function SelectField<T extends string>({
  label,
  aide,
  value,
  options,
  onChange,
}: BaseProps & { value: T; options: Option<T>[]; onChange: (v: T) => void }) {
  const id = useId()
  return (
    <Champ label={label} aide={aide} id={id}>
      <select
        id={id}
        value={value}
        onChange={(e) => {
          const o = options.find((x) => x.value === e.target.value)
          if (o) onChange(o.value)
        }}
      >
        {options.map((o) => (
          <option key={o.value} value={o.value}>
            {o.label}
          </option>
        ))}
      </select>
    </Champ>
  )
}

export function Segmented<T extends string | number>({
  label,
  aide,
  value,
  options,
  onChange,
}: BaseProps & { value: T; options: { value: T; label: string }[]; onChange: (v: T) => void }) {
  const id = useId()
  return (
    <div className="champ" role="radiogroup" aria-labelledby={id}>
      <span className="label" id={id}>
        {label}
      </span>
      <div className="segmente">
        {options.map((o) => (
          <button
            key={String(o.value)}
            type="button"
            role="radio"
            aria-checked={o.value === value}
            className={o.value === value ? 'actif' : ''}
            onClick={() => onChange(o.value)}
          >
            {o.label}
          </button>
        ))}
      </div>
      {aide && <p className="aide">{aide}</p>}
    </div>
  )
}

export function Toggle({ label, aide, value, onChange }: BaseProps & { value: boolean; onChange: (v: boolean) => void }) {
  const id = useId()
  return (
    <div className="champ champ-toggle">
      <label className="toggle" htmlFor={id}>
        <input id={id} type="checkbox" checked={value} onChange={(e) => onChange(e.target.checked)} />
        <span className="piste" aria-hidden="true" />
        <span>{label}</span>
      </label>
      {aide && <p className="aide">{aide}</p>}
    </div>
  )
}

export function TextField({ label, value, onChange }: BaseProps & { value: string; onChange: (v: string) => void }) {
  const id = useId()
  return (
    <Champ label={label} id={id}>
      <input id={id} value={value} onChange={(e) => onChange(e.target.value)} />
    </Champ>
  )
}
