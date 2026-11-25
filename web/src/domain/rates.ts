import type { Varz } from './types'

export interface RatePoint {
  t: number
  inMsgs: number
  outMsgs: number
  inBytes: number
  outBytes: number
}

/**
 * Per-second rates between two `/varz` samples, using the server's own `now` timestamps so
 * polling jitter doesn't distort the rate. Returns null when the samples can't be compared
 * (missing data, same sample twice, or a server restart that reset the counters).
 */
export function rateBetween(prev: Varz | null, cur: Varz | null, t: number): RatePoint | null {
  if (!prev?.now || !cur?.now || prev.now === cur.now) return null
  const dt = (Date.parse(cur.now) - Date.parse(prev.now)) / 1000
  if (!(dt > 0)) return null
  const rate = (k: 'in_msgs' | 'out_msgs' | 'in_bytes' | 'out_bytes') => {
    const a = prev[k]
    const b = cur[k]
    if (a === undefined || b === undefined || b < a) return 0
    return Math.round((b - a) / dt)
  }
  return { t, inMsgs: rate('in_msgs'), outMsgs: rate('out_msgs'), inBytes: rate('in_bytes'), outBytes: rate('out_bytes') }
}

/** Appends a point and keeps only the last `max` points. Returns a new array. */
export function pushWindow<T>(points: readonly T[], point: T, max: number): T[] {
  const next = [...points, point]
  return next.length > max ? next.slice(next.length - max) : next
}
