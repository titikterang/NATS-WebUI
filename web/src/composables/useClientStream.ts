import { onScopeDispose, ref, shallowRef } from 'vue'
import { prependCapped, toRow, type MessageRow } from '@/domain/message'
import type { ClientId, Frame, MsgFrame, NatsStatus } from '@/domain/types'
import { openSocket, type SocketFactory, type SocketHandle } from '@/infrastructure/ws'

export interface ClientStreamOptions {
  /** Rows kept in the list (newest first). */
  max?: number
  /** Incoming messages are batched and rendered at most this often. */
  flushMs?: number
  factory?: SocketFactory
  baseUrl?: string
}

const MAX_ERRORS = 5

/**
 * Live messages for one saved client over `/client/<id>`. Batches renders so a busy subject
 * doesn't re-render on every message; `paused` keeps counting but stops adding rows.
 */
export function useClientStream(opts: ClientStreamOptions = {}) {
  const max = opts.max ?? 500
  const flushMs = opts.flushMs ?? 300

  const running = ref(false)
  const status = ref<NatsStatus | null>(null)
  const messages = shallowRef<MessageRow[]>([])
  const errors = ref<string[]>([])
  const received = ref(0)
  const paused = ref(false)

  let socket: SocketHandle | null = null
  let buffer: MsgFrame[] = []
  let seq = 0
  let timer: ReturnType<typeof setInterval> | null = null

  function flush() {
    if (!buffer.length) return
    const rows = buffer.map((f) => toRow(f, ++seq))
    buffer = []
    messages.value = prependCapped(messages.value, rows, max)
  }

  function onFrame(frame: Frame) {
    switch (frame.typ) {
      case 'Msg':
        received.value++
        if (!paused.value) buffer.push(frame)
        break
      case 'Status':
        status.value = { state: frame.state, error: frame.error }
        break
      case 'Err':
        errors.value = [frame.message, ...errors.value].slice(0, MAX_ERRORS)
        break
    }
  }

  function start(clientId: ClientId) {
    stop()
    errors.value = []
    socket = openSocket<Frame>(`/client/${clientId}`, { onMessage: onFrame, factory: opts.factory, baseUrl: opts.baseUrl })
    timer = setInterval(flush, flushMs)
    running.value = true
  }

  function stop() {
    socket?.close()
    socket = null
    if (timer) clearInterval(timer)
    timer = null
    flush()
    buffer = []
    running.value = false
    status.value = null
  }

  function clear() {
    messages.value = []
    buffer = []
  }

  function dismissError(i: number) {
    errors.value = errors.value.filter((_, j) => j !== i)
  }

  onScopeDispose(stop)

  return { running, status, messages, errors, received, paused, start, stop, clear, dismissError }
}
