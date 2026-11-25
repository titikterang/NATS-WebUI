import axios, { AxiosError } from 'axios'

/** Every failed API call is turned into this, so callers never deal with axios internals. */
export class ApiError extends Error {
  constructor(
    message: string,
    readonly status: number | null,
  ) {
    super(message)
    this.name = 'ApiError'
  }
}

export const http = axios.create({ baseURL: '/api/state', timeout: 15_000 })

http.interceptors.response.use(
  (r) => r,
  (e: AxiosError<{ error?: string }>) => {
    const status = e.response?.status ?? null
    const message = e.response?.data?.error ?? (status ? `request failed (${status})` : e.message || 'network error')
    return Promise.reject(new ApiError(message, status))
  },
)

/** A human-readable message for any thrown value. */
export function errorMessage(e: unknown): string {
  if (e instanceof Error) return e.message
  return String(e)
}
