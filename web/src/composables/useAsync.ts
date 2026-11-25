import { ref, shallowRef } from 'vue'
import { errorMessage } from '@/infrastructure/http'

/** Loading/error/data state around one async call. `run` never throws. */
export function useAsync<T, A extends unknown[]>(fn: (...args: A) => Promise<T>) {
  const data = shallowRef<T | null>(null)
  const error = ref<string | null>(null)
  const loading = ref(false)

  async function run(...args: A): Promise<T | null> {
    loading.value = true
    error.value = null
    try {
      data.value = await fn(...args)
      return data.value
    } catch (e) {
      error.value = errorMessage(e)
      return null
    } finally {
      loading.value = false
    }
  }

  return { data, error, loading, run }
}
