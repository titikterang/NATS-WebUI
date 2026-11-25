import { describe, expect, it } from 'vitest'
import { bytes, num, pct, seconds } from './format'
import { matchesFilter, prependCapped, toRow } from './message'
import { pushWindow, rateBetween } from './rates'
import { percentOf, resources, usageLevel } from './resources'
import { monitorView, natsView } from './status'
import { hostError, isWildcard, publishSubjectError, subjectError } from './subject'
import type { MsgFrame, ServerStatus } from './types'

describe('format', () => {
  it('formats numbers, bytes, percents and durations', () => {
    expect(num(1234)).toBe('1.2K')
    expect(num(undefined)).toBe('—')
    expect(bytes(999)).toBe('999 B')
    expect(bytes(21_474_836_480)).toBe('21.5 GB')
    expect(pct(12.345)).toBe('12.3%')
    expect(seconds(120_000_000_000)).toBe('120s')
  })
})

describe('subject rules (mirror the Rust SubjectPattern)', () => {
  it.each(['a', 'a.b.c', 'a.*.c', 'a.>', '>', '$SYS.>'])('accepts %s', (s) => {
    expect(subjectError(s)).toBeNull()
  })
  it.each(['', ' ', 'a..b', '.a', 'a.', 'a b', 'a.>.b', 'a.b*'])('rejects %j', (s) => {
    expect(subjectError(s)).not.toBeNull()
  })
  it('refuses wildcards when publishing', () => {
    expect(isWildcard('test.>')).toBe(true)
    expect(publishSubjectError('test.>')).toMatch(/wildcard/)
    expect(publishSubjectError('test.x')).toBeNull()
  })
  it('validates hosts like the backend', () => {
    expect(hostError('10.0.0.1')).toBeNull()
    expect(hostError('nats://10.0.0.1')).not.toBeNull()
    expect(hostError('')).not.toBeNull()
  })
})

describe('status views', () => {
  const status = (over: Partial<ServerStatus>): ServerStatus => ({
    server_id: 1,
    nats: { state: 'connected', error: null },
    varz: {},
    varz_error: null,
    ...over,
  })
  it('keeps NATS and monitoring health separate', () => {
    const s = status({ nats: { state: 'auth_failed', error: 'nats: authorization violation' } })
    expect(natsView(s)).toEqual({ label: 'Auth failed', type: 'danger', error: 'nats: authorization violation' })
    expect(monitorView(s).label).toBe('OK')
    expect(monitorView(status({ varz: null, varz_error: 'unreachable' }))).toMatchObject({ label: 'Unreachable', error: 'unreachable' })
    expect(natsView(null).label).toBe('Connecting')
  })
})

describe('resources', () => {
  it('derives JetStream disk figures from varz', () => {
    const r = resources({ cpu: 1, cores: 2, mem: 10, jetstream: { config: { max_storage: 100 }, stats: { storage: 25 } } })
    expect(r).toMatchObject({ jsEnabled: true, diskUsed: 25, diskFree: 75 })
    expect(resources({})?.jsEnabled).toBe(false)
    expect(resources(null)).toBeNull()
  })
  it('computes clamped percentages and levels', () => {
    expect(percentOf(25, 100)).toBe(25)
    expect(percentOf(5, 0)).toBe(0)
    expect(percentOf(200, 100)).toBe(100)
    expect([usageLevel(10), usageLevel(80), usageLevel(95)]).toEqual(['ok', 'warn', 'critical'])
  })
})

describe('rates', () => {
  const v = (now: string, inMsgs: number) => ({ now, in_msgs: inMsgs, out_msgs: 0, in_bytes: 0, out_bytes: 0 })
  it('uses server timestamps', () => {
    const r = rateBetween(v('2026-01-01T00:00:00Z', 0), v('2026-01-01T00:00:02Z', 10), 1)
    expect(r?.inMsgs).toBe(5)
  })
  it('ignores repeated samples and counter resets', () => {
    expect(rateBetween(v('2026-01-01T00:00:00Z', 0), v('2026-01-01T00:00:00Z', 10), 1)).toBeNull()
    expect(rateBetween(v('2026-01-01T00:00:00Z', 50), v('2026-01-01T00:00:01Z', 10), 1)?.inMsgs).toBe(0)
  })
  it('keeps a sliding window', () => {
    expect(pushWindow([1, 2, 3], 4, 3)).toEqual([2, 3, 4])
  })
})

describe('messages', () => {
  const frame = (payload: string, extra: Partial<MsgFrame> = {}): MsgFrame => ({
    typ: 'Msg', ts: 0, subject: 'test.a', reply: null, headers: {}, size: payload.length, payload, truncated: false, ...extra,
  })
  it('pretty-prints JSON and flattens headers', () => {
    const row = toRow(frame('{"a":1}', { headers: { X: ['1', '2'] } }), 1)
    expect(row.isJson).toBe(true)
    expect(row.pretty).toContain('\n')
    expect(row.headers).toEqual([['X', '1'], ['X', '2']])
    expect(toRow(frame('plain'), 2).isJson).toBe(false)
  })
  it('keeps newest first and caps the list', () => {
    const rows = [1, 2, 3].map((k) => toRow(frame(`m${k}`), k))
    const next = prependCapped([rows[0]!], [rows[1]!, rows[2]!], 2)
    expect(next.map((r) => r.key)).toEqual([3, 2])
  })
  it('filters on subject and payload', () => {
    const row = toRow(frame('hello world'), 1)
    expect(matchesFilter(row, 'WORLD')).toBe(true)
    expect(matchesFilter(row, 'test.a')).toBe(true)
    expect(matchesFilter(row, 'nope')).toBe(false)
    expect(matchesFilter(row, '  ')).toBe(true)
  })
})
