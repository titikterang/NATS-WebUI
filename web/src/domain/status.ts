import type { ConnState, NatsStatus, ServerStatus } from './types'

export type TagType = 'success' | 'info' | 'danger' | 'warning'

export interface StateView {
  label: string
  type: TagType
  error: string | null
}

const NATS: Record<ConnState, Omit<StateView, 'error'>> = {
  connected: { label: 'Connected', type: 'success' },
  connecting: { label: 'Connecting', type: 'info' },
  auth_failed: { label: 'Auth failed', type: 'danger' },
  error: { label: 'Error', type: 'danger' },
}

/** How an authenticated NATS connection is shown. No status yet means "Connecting". */
export function connView(nats: NatsStatus | null | undefined): StateView {
  return { ...NATS[nats?.state ?? 'connecting'], error: nats?.error ?? null }
}

/** `connView` for a server's status-feed entry. */
export function natsView(status: ServerStatus | null | undefined): StateView {
  return connView(status?.nats)
}

/** How the plain-HTTP monitoring endpoint is shown. */
export function monitorView(status: ServerStatus | null | undefined): StateView {
  if (!status) return { label: 'Waiting', type: 'info', error: null }
  return status.varz
    ? { label: 'OK', type: 'success', error: null }
    : { label: 'Unreachable', type: 'danger', error: status.varz_error }
}
