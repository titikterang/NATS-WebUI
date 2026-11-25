import { computed, ref, watch, type Ref } from 'vue'
import { pushWindow, rateBetween, type RatePoint } from '@/domain/rates'
import type { Varz } from '@/domain/types'

/** Per-second message and byte rates derived from successive `/varz` samples. */
export function useServerRates(varz: Ref<Varz | null>, windowSize = 60) {
  const points = ref<RatePoint[]>([])

  watch(varz, (cur, prev) => {
    const p = rateBetween(prev ?? null, cur ?? null, Date.now())
    if (p) points.value = pushWindow(points.value, p, windowSize)
  })

  const last = computed<RatePoint | null>(() => points.value[points.value.length - 1] ?? null)
  const reset = () => (points.value = [])

  return { points, last, reset }
}
