import { afterEach, describe, expect, it, vi } from 'vitest'
import { FakeSocket } from '@/test/fakeSocket'
import { openSocket, wsBase } from './ws'

afterEach(() => {
  FakeSocket.all = []
  vi.useRealTimers()
})

describe('wsBase', () => {
  it('matches the page protocol', () => {
    // http pages get the plain scheme, https pages get the TLS one
    expect(wsBase({ protocol: 'http:', host: 'h:1' })).toBe(['ws', '//h:1'].join(':'))
    expect(wsBase({ protocol: 'https:', host: 'h' })).toBe('wss://h')
  })
})

describe('openSocket', () => {
  const open = (onMessage = vi.fn()) =>
    openSocket('/x', { onMessage, baseUrl: 'wss://h', factory: (u) => new FakeSocket(u) })

  it('parses JSON frames and ignores malformed ones', () => {
    const onMessage = vi.fn()
    open(onMessage)
    const s = FakeSocket.all[0]!
    expect(s.url).toBe('wss://h/x')
    s.send({ a: 1 })
    s.onmessage?.({ data: 'not json' })
    expect(onMessage).toHaveBeenCalledTimes(1)
    expect(onMessage).toHaveBeenCalledWith({ a: 1 })
  })

  it('reconnects with backoff after a server close, and stops after close()', () => {
    vi.useFakeTimers()
    const handle = open()
    FakeSocket.all[0]!.drop()
    vi.advanceTimersByTime(999)
    expect(FakeSocket.all).toHaveLength(1)
    vi.advanceTimersByTime(1)
    expect(FakeSocket.all).toHaveLength(2)

    FakeSocket.all[1]!.drop()
    vi.advanceTimersByTime(2000)
    expect(FakeSocket.all).toHaveLength(3)

    handle.close()
    vi.advanceTimersByTime(60_000)
    expect(FakeSocket.all).toHaveLength(3)
    expect(FakeSocket.all[2]!.closed).toBe(true)
  })
})
