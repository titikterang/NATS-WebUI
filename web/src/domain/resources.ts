import type { Varz } from './types'

/**
 * Resource figures available from `/varz`. CPU and RSS describe the nats-server process. Disk and
 * memory limits are JetStream limits; by default nats-server sets the disk limit to 75% of the
 * free disk space at startup. Host-level figures need a separate agent.
 */
export interface Resources {
  cpu: number | undefined
  cores: number | undefined
  rss: number | undefined
  jsEnabled: boolean
  memUsed: number
  memMax: number
  diskUsed: number
  diskMax: number
  diskFree: number
  storeDir: string | undefined
}

export function resources(varz: Varz | null | undefined): Resources | null {
  if (!varz) return null
  const cfg = varz.jetstream?.config
  const stats = varz.jetstream?.stats
  const diskMax = cfg?.max_storage ?? 0
  const diskUsed = stats?.storage ?? 0
  return {
    cpu: varz.cpu,
    cores: varz.cores,
    rss: varz.mem,
    jsEnabled: cfg !== undefined,
    memUsed: stats?.memory ?? 0,
    memMax: cfg?.max_memory ?? 0,
    diskUsed,
    diskMax,
    diskFree: Math.max(0, diskMax - diskUsed),
    storeDir: cfg?.store_dir,
  }
}

/** Used / max as a percentage with one decimal, clamped to 0–100. 0 when there is no limit. */
export function percentOf(used: number, max: number): number {
  if (!max) return 0
  return Math.min(100, Math.max(0, Math.round((used / max) * 1000) / 10))
}

/** Traffic-light thresholds for usage bars. */
export function usageLevel(percent: number): 'ok' | 'warn' | 'critical' {
  return percent < 70 ? 'ok' : percent < 90 ? 'warn' : 'critical'
}
