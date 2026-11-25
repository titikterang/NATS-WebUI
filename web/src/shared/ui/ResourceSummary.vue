<script setup lang="ts">
import { computed } from 'vue'
import { bytes, pct } from '@/domain/format'
import { percentOf, resources, usageLevel } from '@/domain/resources'
import type { Varz } from '@/domain/types'

/** Compact one-line-per-resource summary for table rows. */
const props = defineProps<{ varz: Varz | null }>()
const r = computed(() => resources(props.varz))
const diskPct = computed(() => (r.value ? percentOf(r.value.diskUsed, r.value.diskMax) : 0))
const COLORS = { ok: '#67C23A', warn: '#E6A23C', critical: '#F56C6C' }
</script>

<template>
  <span v-if="!r" class="muted">no monitoring data</span>
  <div v-else class="summary">
    <div><span class="k">CPU</span>{{ pct(r.cpu) }} <span class="muted">/ {{ r.cores }} cores</span></div>
    <div><span class="k">RAM</span>{{ bytes(r.rss) }}</div>
    <div v-if="r.jsEnabled" class="disk">
      <span class="k">Disk</span>
      <el-progress :percentage="diskPct" :stroke-width="6" :show-text="false" :color="COLORS[usageLevel(diskPct)]" class="bar" />
      <span class="muted">{{ bytes(r.diskFree) }} free</span>
    </div>
  </div>
</template>

<style scoped>
.summary {
  font-size: 12px;
  line-height: 18px;
}
.k {
  display: inline-block;
  width: 34px;
  color: var(--el-text-color-secondary);
}
.disk {
  display: flex;
  align-items: center;
  gap: 6px;
}
.bar {
  width: 60px;
}
</style>
