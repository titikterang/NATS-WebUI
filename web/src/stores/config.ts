import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { api } from '@/infrastructure/api'
import type { Client, ClientId, ClientInput, Server, ServerId, ServerInput } from '@/domain/types'

/** Servers and clients: the persisted configuration. Every mutation reloads from the backend. */
export const useConfigStore = defineStore('config', () => {
  const servers = ref<Server[]>([])
  const clients = ref<Client[]>([])
  const loaded = ref(false)

  const serverById = computed(() => new Map(servers.value.map((s) => [s.id, s])))
  const clientById = computed(() => new Map(clients.value.map((c) => [c.id, c])))

  async function refresh() {
    const state = await api.state()
    servers.value = state.servers
    clients.value = state.clients
    loaded.value = true
  }

  async function saveServer(input: ServerInput): Promise<Server> {
    const saved = input.id ? await api.updateServer(input) : await api.createServer(input)
    await refresh()
    return saved
  }

  async function deleteServer(id: ServerId) {
    await api.deleteServer(id)
    await refresh()
  }

  async function saveClient(input: ClientInput): Promise<Client> {
    const saved = input.id ? await api.updateClient(input) : await api.createClient(input)
    await refresh()
    return saved
  }

  async function deleteClient(id: ClientId) {
    await api.deleteClient(id)
    await refresh()
  }

  return { servers, clients, loaded, serverById, clientById, refresh, saveServer, deleteServer, saveClient, deleteClient }
})
