// Number and time formatting, shared by every view (no formatting logic in components).

const DASH = '—'
const compact = new Intl.NumberFormat('en', { notation: 'compact', maximumFractionDigits: 1 })
const oneDecimal = new Intl.NumberFormat('en', { maximumFractionDigits: 1 })

const isMissing = (v: number | null | undefined): v is null | undefined =>
  v === null || v === undefined || Number.isNaN(v)

/** 1234 → "1.2K". */
export function num(v: number | null | undefined): string {
  return isMissing(v) ? DASH : compact.format(v)
}

/** Decimal (SI) bytes, matching how nats-server reports sizes: 1500 → "1.5 KB". */
export function bytes(v: number | null | undefined): string {
  if (isMissing(v)) return DASH
  const units = ['B', 'KB', 'MB', 'GB', 'TB', 'PB']
  let value = v
  let unit = 0
  while (Math.abs(value) >= 1000 && unit < units.length - 1) {
    value /= 1000
    unit++
  }
  return `${oneDecimal.format(value)} ${units[unit]}`
}

export function pct(v: number | null | undefined): string {
  return isMissing(v) ? DASH : `${oneDecimal.format(v)}%`
}

/** Nanoseconds (as nats-server reports durations) → "120s". */
export function seconds(ns: number | null | undefined): string {
  return isMissing(ns) ? DASH : `${oneDecimal.format(ns / 1e9)}s`
}

/** Epoch millis → local "HH:MM:SS.mmm". */
export function clock(ts: number): string {
  const d = new Date(ts)
  const pad = (n: number, w = 2) => String(n).padStart(w, '0')
  return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}.${pad(d.getMilliseconds(), 3)}`
}

/** Shows a value or a dash for missing/empty values. */
export function orDash(v: unknown): string {
  return v === null || v === undefined || v === '' ? DASH : String(v)
}
