<script setup lang="ts">
import { computed } from 'vue'
import { bytes, pct } from '@/domain/format'
import { percentOf, resources, usageLevel } from '@/domain/resources'
import type { Varz } from '@/domain/types'

/** Detailed resource cards for the expanded row and the dashboard. */
const props = defineProps<{ varz: Varz | null }>()
const r = computed(() => resources(props.varz))
const COLORS = { ok: '#67C23A', warn: '#E6A23C', critical: '#F56C6C' }
const color = (p: number) => COLORS[usageLevel(p)]
</script>

<template>
  <el-empty v-if="!r" description="No monitoring data" :image-size="60" />
  <div v-else class="cards">
    <el-card shadow="never">
      <div class="label">CPU (nats-server)</div>
      <el-progress :percentage="Math.min(100, r.cpu ?? 0)" :color="color(r.cpu ?? 0)" />
      <div class="muted">{{ pct(r.cpu) }} across {{ r.cores }} cores</div>
    </el-card>
    <el-card shadow="never">
      <div class="label">RAM (nats-server RSS)</div>
      <div class="big">{{ bytes(r.rss) }}</div>
      <div v-if="r.jsEnabled" class="muted">JetStream memory {{ bytes(r.memUsed) }} / {{ bytes(r.memMax) }}</div>
    </el-card>
    <el-card shadow="never">
      <div class="label">Disk (JetStream store)</div>
      <template v-if="r.jsEnabled">
        <el-progress :percentage="percentOf(r.diskUsed, r.diskMax)" :color="color(percentOf(r.diskUsed, r.diskMax))" />
        <div class="muted">
          {{ bytes(r.diskUsed) }} used · <b>{{ bytes(r.diskFree) }} remaining</b> of {{ bytes(r.diskMax) }}
        </div>
        <div class="muted mono path">{{ r.storeDir }}</div>
      </template>
      <div v-else class="muted">JetStream disabled: no storage figures</div>
    </el-card>
    <p class="note muted">
      From the NATS monitoring endpoint. CPU and RAM are for the nats-server process. The disk limit is JetStream's
      <code>max_file_store</code>, which defaults to 75% of the free disk at server start.
    </p>
  </div>
</template>

<style scoped>
.cards {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 12px;
}
.label {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  margin-bottom: 6px;
}
.big {
  font-size: 20px;
  font-weight: 300;
}
.path {
  word-break: break-all;
}
.note {
  grid-column: 1 / -1;
  margin: 0;
}
</style>
