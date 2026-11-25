import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { effectScope } from 'vue'
import { FakeSocket } from '@/test/fakeSocket'
import { useClientStream } from './useClientStream'

const msg = (subject: string) => ({ typ: 'Msg', ts: 0, subject, reply: null, headers: {}, size: 1, payload: 'x', truncated: false })

describe('useClientStream', () => {
  beforeEach(() => vi.useFakeTimers())
  afterEach(() => {
    FakeSocket.all = []
    vi.useRealTimers()
  })

  function setup(max = 3) {
    const scope = effectScope()
    const stream = scope.run(() =>
      useClientStream({ max, flushMs: 100, baseUrl: 'wss://h', factory: (u) => new FakeSocket(u) }),
    )!
    return { scope, stream }
  }

  it('opens /client/<id>, batches messages newest-first and caps the list', () => {
    const { stream } = setup()
    stream.start(7)
    const s = FakeSocket.all[0]!
    expect(s.url).toBe('wss://h/client/7')

    for (const n of ['a', 'b', 'c', 'd']) s.send(msg(n))
    expect(stream.messages.value).toHaveLength(0) // not flushed yet
    vi.advanceTimersByTime(100)
    expect(stream.messages.value.map((m) => m.subject)).toEqual(['d', 'c', 'b'])
    expect(stream.received.value).toBe(4)
  })

  it('tracks status and keeps the latest errors', () => {
    const { stream } = setup()
    stream.start(1)
    const s = FakeSocket.all[0]!
    s.send({ typ: 'Status', ts: 0, state: 'auth_failed', error: 'nope' })
    for (let i = 0; i < 7; i++) s.send({ typ: 'Err', ts: 0, message: `e${i}` })
    expect(stream.status.value).toEqual({ state: 'auth_failed', error: 'nope' })
    expect(stream.errors.value).toEqual(['e6', 'e5', 'e4', 'e3', 'e2'])
  })

  it('counts but does not add rows while paused', () => {
    const { stream } = setup()
    stream.start(1)
    stream.paused.value = true
    FakeSocket.all[0]!.send(msg('a'))
    vi.advanceTimersByTime(100)
    expect(stream.received.value).toBe(1)
    expect(stream.messages.value).toHaveLength(0)
  })

  it('closes the socket when stopped or when the owning scope is disposed', () => {
    const { scope, stream } = setup()
    stream.start(1)
    stream.start(2) // restarting closes the previous socket
    expect(FakeSocket.all[0]!.closed).toBe(true)
    scope.stop()
    expect(FakeSocket.all[1]!.closed).toBe(true)
    expect(stream.running.value).toBe(false)
  })
})
