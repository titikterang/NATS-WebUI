<script setup lang="ts">
import { computed } from 'vue'
import { bytes, num, orDash, pct, seconds } from '@/domain/format'
import type { Server } from '@/domain/types'
import { useServerRates } from '@/composables/useServerRates'
import { useStatusStore } from '@/stores/status'
import ResourceCards from '@/shared/ui/ResourceCards.vue'

const props = defineProps<{ server: Server }>()
const status = useStatusStore()
const varz = computed(() => status.of(props.server.id)?.varz ?? null)
const varzError = computed(() => status.of(props.server.id)?.varz_error ?? 'waiting for data')
const { points, last } = useServerRates(varz)

const stats = computed(() => {
  const v = varz.value ?? {}
  return [
    ['Msgs in / s', num(last.value?.inMsgs)],
    ['Msgs out / s', num(last.value?.outMsgs)],
    ['In / s', bytes(last.value?.inBytes)],
    ['Out / s', bytes(last.value?.outBytes)],
    ['Connections', num(v.connections)],
    ['Subscriptions', num(v.subscriptions)],
    ['CPU', pct(v.cpu)],
    ['Uptime', orDash(v.uptime)],
  ]
})

const config = computed<[string, string][]>(() => {
  const v = varz.value ?? {}
  return [
    ['Server name', orDash(v.server_name)], ['Server ID', orDash(v.server_id)], ['Version', orDash(v.version)],
    ['Go', orDash(v.go)], ['Protocol', orDash(v.proto)], ['Cores', orDash(v.cores)],
    ['Cluster', orDash(v.cluster?.name)], ['Routes', orDash(v.routes)], ['Leaf nodes', orDash(v.leafnodes)],
    ['Max connections', num(v.max_connections)], ['Max payload', bytes(v.max_payload)], ['Max pending', bytes(v.max_pending)],
    ['Ping interval', seconds(v.ping_interval)], ['Ping max', orDash(v.ping_max)], ['Write deadline', seconds(v.write_deadline)],
    ['TLS required', v.tls_required ? 'yes' : 'no'], ['TLS cert expires', orDash(v.tls_cert_not_after)],
    ['Auth required', v.auth_required ? 'yes' : 'no'],
    ['Total msgs in / out', `${num(v.in_msgs)} / ${num(v.out_msgs)}`],
    ['Total data in / out', `${bytes(v.in_bytes)} / ${bytes(v.out_bytes)}`],
    ['Slow consumers', orDash(v.slow_consumers)], ['Started', orDash(v.start)], ['Config loaded', orDash(v.config_load_time)],
  ]
})

const series = (a: 'inMsgs' | 'inBytes', b: 'outMsgs' | 'outBytes') => [
  { name: 'In', data: points.value.map((p) => [p.t, p[a]]) },
  { name: 'Out', data: points.value.map((p) => [p.t, p[b]]) },
]
const chart = (title: string, fmt: (v: number) => string) => ({
  chart: { animations: { enabled: false }, toolbar: { show: false }, zoom: { enabled: false } },
  title: { text: title, style: { fontSize: '12px', color: '#909399' } },
  dataLabels: { enabled: false },
  stroke: { curve: 'smooth', width: 2 },
  xaxis: { type: 'datetime', labels: { show: false } },
  yaxis: { min: 0, labels: { formatter: fmt } },
  legend: { position: 'top', horizontalAlign: 'right' },
  tooltip: { x: { format: 'HH:mm:ss' } },
})
const msgChart = chart('Messages / s', num)
const byteChart = chart('Bytes / s', bytes)
</script>

<template>
  <div v-if="varz">
    <div class="stats">
      <el-card v-for="[label, value] in stats" :key="label" shadow="never" body-class="stat">
        <div class="muted">{{ label }}</div>
        <div class="value">{{ value }}</div>
      </el-card>
    </div>
    <div class="charts">
      <apexchart type="area" height="170" :options="msgChart" :series="series('inMsgs', 'outMsgs')" />
      <apexchart type="area" height="170" :options="byteChart" :series="series('inBytes', 'outBytes')" />
    </div>
    <h4>Resources</h4>
    <ResourceCards :varz="varz" />
    <h4>Configuration</h4>
    <el-descriptions :column="3" border size="small">
      <el-descriptions-item v-for="[k, val] in config" :key="k" :label="k">{{ val }}</el-descriptions-item>
    </el-descriptions>
  </div>
  <el-alert v-else type="error" :closable="false" show-icon :title="`Monitoring endpoint unavailable: ${varzError}`" />
</template>

<style scoped>
.stats {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
  gap: 8px;
}
.stats :deep(.stat) {
  padding: 8px 10px;
}
.value {
  font-size: 20px;
  font-weight: 300;
}
.charts {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
  margin-top: 8px;
}
h4 {
  margin: 16px 0 8px;
}
</style>
