import { defineStore } from 'pinia'
import { ref } from 'vue'
import { openSocket, type SocketHandle } from '@/infrastructure/ws'
import type { ServerId, ServerStatus } from '@/domain/types'

/** Live status per server, fed by the `/api/state/ws` websocket (one update per server per second). */
export const useStatusStore = defineStore('status', () => {
  const byServer = ref<Record<ServerId, ServerStatus>>({})
  const online = ref(false)
  let socket: SocketHandle | null = null

  function start() {
    if (socket) return
    socket = openSocket<ServerStatus>('/api/state/ws', {
      onMessage: (s) => {
        byServer.value[s.server_id] = s
      },
      onOpen: () => (online.value = true),
      onClose: () => (online.value = false),
    })
  }

  function stop() {
    socket?.close()
    socket = null
  }

  const of = (id: ServerId): ServerStatus | null => byServer.value[id] ?? null

  return { byServer, online, start, stop, of }
})
