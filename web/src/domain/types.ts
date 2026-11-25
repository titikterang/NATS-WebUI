// The API contract. Mirrors src/presentation/http/dto.rs in the Rust backend: change both together.

export type ServerId = number
export type ClientId = number

/** A server as the API returns it. The password is never sent; `has_password` says one is stored. */
export interface Server {
  id: ServerId
  name: string
  host: string
  port: number
  monitoring_port: number
  username: string | null
  has_password: boolean
}

/** Create/update body. On update, `password: null` keeps the stored password. */
export interface ServerInput {
  id?: ServerId
  name: string
  host: string
  port: number
  monitoring_port: number
  username: string | null
  password: string | null
}

/** A saved subscription view: subject patterns to watch on one server. */
export interface Client {
  id: ClientId
  name: string
  server_id: ServerId
  subjects: string[]
}

export type ClientInput = Omit<Client, 'id'> & { id?: ClientId }

export interface AppState {
  servers: Server[]
  clients: Client[]
}

export type ConnState = 'connected' | 'connecting' | 'auth_failed' | 'error'

export interface NatsStatus {
  state: ConnState
  error: string | null
}

/** Fields of nats-server `/varz` that the UI reads. Everything is optional: versions differ. */
export interface Varz {
  server_id?: string
  server_name?: string
  version?: string
  go?: string
  proto?: number
  now?: string
  start?: string
  uptime?: string
  config_load_time?: string
  cores?: number
  cpu?: number
  mem?: number
  connections?: number
  total_connections?: number
  subscriptions?: number
  slow_consumers?: number
  routes?: number
  leafnodes?: number
  in_msgs?: number
  out_msgs?: number
  in_bytes?: number
  out_bytes?: number
  max_connections?: number
  max_payload?: number
  max_pending?: number
  max_control_line?: number
  ping_interval?: number
  ping_max?: number
  write_deadline?: number
  tls_required?: boolean
  tls_timeout?: number
  tls_cert_not_after?: string
  auth_required?: boolean
  cluster?: { name?: string }
  jetstream?: {
    config?: { max_memory?: number; max_storage?: number; store_dir?: string }
    stats?: { memory?: number; storage?: number }
  }
}

/** One tick of the `/api/state/ws` status feed. */
export interface ServerStatus {
  server_id: ServerId
  nats: NatsStatus
  varz: Varz | null
  varz_error: string | null
}

/** Frames on the `/client/<id>` websocket. */
export type Frame =
  | {
      typ: 'Msg'
      ts: number
      subject: string
      reply: string | null
      headers: Record<string, string[]>
      size: number
      payload: string
      truncated: boolean
    }
  | { typ: 'Status'; ts: number; state: ConnState; error: string | null }
  | { typ: 'Err'; ts: number; message: string }

export type MsgFrame = Extract<Frame, { typ: 'Msg' }>

export interface Header {
  name: string
  value: string
}

export interface PublishRequest {
  subject: string
  payload: string
  headers: Header[]
}

/** `/jsz?streams=true&consumers=true` (subset). */
export interface Jsz {
  streams?: number
  consumers?: number
  messages?: number
  bytes?: number
  account_details?: {
    name: string
    stream_detail?: StreamInfo[]
  }[]
}

export interface StreamInfo {
  name: string
  config?: { subjects?: string[]; storage?: string }
  state: { messages: number; bytes: number; consumer_count: number; last_seq: number }
  consumer_detail?: ConsumerInfo[]
}

export interface ConsumerInfo {
  name: string
  num_pending?: number
  num_ack_pending?: number
  num_redelivered?: number
  delivered?: { stream_seq?: number }
}

/** `/subsz?subs=1` (subset). */
export interface Subsz {
  num_subscriptions?: number
  subscriptions_list?: SubscriptionInfo[]
}

export interface SubscriptionInfo {
  subject: string
  qgroup?: string
  account?: string
  msgs?: number
  cid?: number
}
