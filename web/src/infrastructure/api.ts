import type {
  AppState,
  Client,
  ClientId,
  ClientInput,
  Jsz,
  PublishRequest,
  Server,
  ServerId,
  ServerInput,
  Subsz,
} from '@/domain/types'
import { http } from './http'

// One typed function per backend endpoint (see src/presentation/http/mod.rs).
export const api = {
  state: () => http.get<AppState>('').then((r) => r.data),

  createServer: (s: ServerInput) => http.post<Server>('/server/new', s).then((r) => r.data),
  updateServer: (s: ServerInput) => http.post<Server>('/server/update', s).then((r) => r.data),
  deleteServer: (id: ServerId) => http.get(`/server/delete/${id}`).then(() => undefined),

  jsz: (id: ServerId) => http.get<Jsz>(`/server/${id}/jsz`).then((r) => r.data),
  subsz: (id: ServerId) => http.get<Subsz>(`/server/${id}/subsz`).then((r) => r.data),
  publish: (id: ServerId, msg: PublishRequest) => http.post(`/server/${id}/publish`, msg).then(() => undefined),

  createClient: (c: ClientInput) => http.post<Client>('/client/new', c).then((r) => r.data),
  updateClient: (c: ClientInput) => http.post<Client>('/client/update', c).then((r) => r.data),
  deleteClient: (id: ClientId) => http.get(`/client/delete/${id}`).then(() => undefined),
}

export type Api = typeof api
