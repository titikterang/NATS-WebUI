/** Minimal surface of a WebSocket, so tests can pass a fake. */
export interface SocketLike {
  onopen: ((ev: unknown) => void) | null
  onmessage: ((ev: { data: unknown }) => void) | null
  onclose: ((ev: unknown) => void) | null
  close(): void
}

export type SocketFactory = (url: string) => SocketLike

export interface SocketHandle {
  close(): void
}

export interface SocketOptions<T> {
  onMessage: (msg: T) => void
  onOpen?: () => void
  onClose?: () => void
  /** Reconnect after the server closes the socket (default true). */
  reconnect?: boolean
  factory?: SocketFactory
  baseUrl?: string
}

/** The websocket origin matching the page: http pages get ws, https pages get wss. */
export function wsBase(location: { protocol: string; host: string } = window.location): string {
  return `${location.protocol.replace(/^http/, 'ws')}//${location.host}`
}

const MAX_DELAY_MS = 10_000

/**
 * Opens a JSON websocket that reconnects with exponential backoff (1s, 2s, 4s … 10s) until
 * `close()` is called. Replaces the unmaintained reconnecting-websocket package.
 */
export function openSocket<T>(path: string, opts: SocketOptions<T>): SocketHandle {
  const factory = opts.factory ?? ((url: string) => new WebSocket(url) as unknown as SocketLike)
  const url = (opts.baseUrl ?? wsBase()) + path
  let socket: SocketLike | null = null
  let closed = false
  let attempt = 0
  let timer: ReturnType<typeof setTimeout> | null = null

  const connect = () => {
    socket = factory(url)
    socket.onopen = () => {
      attempt = 0
      opts.onOpen?.()
    }
    socket.onmessage = (ev) => {
      try {
        opts.onMessage(JSON.parse(String(ev.data)) as T)
      } catch {
        // ignore malformed frames
      }
    }
    socket.onclose = () => {
      opts.onClose?.()
      if (closed || opts.reconnect === false) return
      const delay = Math.min(MAX_DELAY_MS, 1000 * 2 ** attempt++)
      timer = setTimeout(connect, delay)
    }
  }
  connect()

  return {
    close() {
      closed = true
      if (timer) clearTimeout(timer)
      socket?.close()
    },
  }
}
