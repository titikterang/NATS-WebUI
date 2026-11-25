import { clock } from './format'
import type { MsgFrame } from './types'

/** A received message, prepared once for display (rows are immutable). */
export interface MessageRow {
  key: number
  ts: number
  time: string
  subject: string
  reply: string | null
  size: number
  truncated: boolean
  headers: [string, string][]
  preview: string
  pretty: string
  isJson: boolean
}

const PREVIEW_CHARS = 200

export function toRow(frame: MsgFrame, key: number): MessageRow {
  let pretty = frame.payload
  let isJson = false
  try {
    pretty = JSON.stringify(JSON.parse(frame.payload), null, 2)
    isJson = true
  } catch {
    // not JSON: show as-is
  }
  const headers: [string, string][] = []
  for (const [name, values] of Object.entries(frame.headers ?? {})) {
    for (const v of values) headers.push([name, v])
  }
  return Object.freeze({
    key,
    ts: frame.ts,
    time: clock(frame.ts),
    subject: frame.subject,
    reply: frame.reply,
    size: frame.size,
    truncated: frame.truncated,
    headers,
    preview: frame.payload.length > PREVIEW_CHARS ? `${frame.payload.slice(0, PREVIEW_CHARS)}…` : frame.payload,
    pretty,
    isJson,
  })
}

/** Case-insensitive match on subject or payload preview. Empty filter matches everything. */
export function matchesFilter(row: MessageRow, filter: string): boolean {
  const f = filter.trim().toLowerCase()
  return !f || row.subject.toLowerCase().includes(f) || row.preview.toLowerCase().includes(f)
}

/** Newest-first list capped at `max`: `incoming` is in arrival order. */
export function prependCapped(rows: readonly MessageRow[], incoming: readonly MessageRow[], max: number): MessageRow[] {
  return [...incoming].reverse().concat(rows).slice(0, max)
}
