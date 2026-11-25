<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Delete, Edit, Plus } from '@element-plus/icons-vue'
import { bytes, num, orDash, seconds } from '@/domain/format'
import { natsView } from '@/domain/status'
import type { Server } from '@/domain/types'
import { errorMessage } from '@/infrastructure/http'
import { useConfigStore } from '@/stores/config'
import { useStatusStore } from '@/stores/status'
import ResourceCards from '@/shared/ui/ResourceCards.vue'
import ResourceSummary from '@/shared/ui/ResourceSummary.vue'
import StatusTags from '@/shared/ui/StatusTags.vue'
import ServerFormDialog from './ServerFormDialog.vue'

const router = useRouter()
const config = useConfigStore()
const status = useStatusStore()

const rows = computed(() =>
  config.servers.map((s) => {
    const st = status.of(s.id)
    return { ...s, status: st, varz: st?.varz ?? null }
  }),
)
type Row = (typeof rows.value)[number]

const formOpen = ref(false)
const editing = ref<Server | null>(null)
function openForm(server: Server | null) {
  editing.value = server
  formOpen.value = true
}

async function remove(s: Server) {
  try {
    await ElMessageBox.confirm(`Delete "${s.name}" and the clients that watch it?`, 'Delete server', {
      type: 'warning',
      confirmButtonText: 'Delete',
    })
  } catch {
    return
  }
  try {
    await config.deleteServer(s.id)
  } catch (e) {
    ElMessage.error(errorMessage(e))
  }
}

const v = (r: Row, k: 'version' | 'uptime') => orDash(r.varz?.[k])

function details(r: Row): [string, string][] {
  const z = r.varz ?? {}
  return [
    ['Address', `${r.host}:${r.port}`],
    ['Monitoring', `http://${r.host}:${r.monitoring_port}`],
    ['Auth user', r.username ?? '(none)'],
    ['Server name', orDash(z.server_name)],
    ['Cluster', orDash(z.cluster?.name)],
    ['Routes', orDash(z.routes)],
    ['Connections', `${num(z.connections)} (total ${num(z.total_connections)})`],
    ['Subscriptions', num(z.subscriptions)],
    ['Slow consumers', orDash(z.slow_consumers)],
    ['Data in / out', `${bytes(z.in_bytes)} / ${bytes(z.out_bytes)}`],
    ['TLS required', z.tls_required ? 'yes' : 'no'],
    ['TLS cert expires', orDash(z.tls_cert_not_after)],
    ['Max payload', bytes(z.max_payload)],
    ['Ping interval', seconds(z.ping_interval)],
    ['Started', orDash(z.start)],
  ]
}
</script>

<template>
  <div class="page-header">
    <h2>Servers</h2>
    <el-button type="primary" :icon="Plus" @click="openForm(null)">Add server</el-button>
  </div>

  <el-table :data="rows" row-key="id" class="clickable" @row-click="(r: Row) => router.push(`/servers/${r.id}`)">
    <template #empty>
      No servers yet. <el-link type="primary" @click="openForm(null)">Add your first server</el-link>
    </template>

    <el-table-column type="expand" width="36">
      <template #default="{ row }">
        <div class="detail" @click.stop>
          <ResourceCards :varz="row.varz" />
          <el-descriptions :column="3" border size="small">
            <el-descriptions-item v-for="[k, val] in details(row)" :key="k" :label="k">{{ val }}</el-descriptions-item>
          </el-descriptions>
          <el-alert v-if="natsView(row.status).error" type="error" :closable="false" :title="`NATS: ${natsView(row.status).error}`" />
          <el-alert v-if="row.status?.varz_error" type="warning" :closable="false" :title="`Monitor: ${row.status.varz_error}`" />
        </div>
      </template>
    </el-table-column>

    <el-table-column label="Name" min-width="150">
      <template #default="{ row }">
        <div class="name">{{ row.name }}</div>
        <div class="mono muted">{{ row.host }}:{{ row.port }}</div>
      </template>
    </el-table-column>
    <el-table-column label="Status" min-width="210">
      <template #default="{ row }"><StatusTags :status="row.status" /></template>
    </el-table-column>
    <el-table-column label="Resources" min-width="200">
      <template #default="{ row }"><ResourceSummary :varz="row.varz" /></template>
    </el-table-column>
    <el-table-column label="Version" width="85">
      <template #default="{ row }">{{ v(row, 'version') }}</template>
    </el-table-column>
    <el-table-column label="Conns" width="75" align="right">
      <template #default="{ row }">{{ num(row.varz?.connections) }}</template>
    </el-table-column>
    <el-table-column label="Msgs in / out" width="125" align="right">
      <template #default="{ row }">{{ num(row.varz?.in_msgs) }} / {{ num(row.varz?.out_msgs) }}</template>
    </el-table-column>
    <el-table-column label="Uptime" width="110">
      <template #default="{ row }">{{ v(row, 'uptime') }}</template>
    </el-table-column>
    <el-table-column width="100" align="right">
      <template #default="{ row }">
        <el-button :icon="Edit" circle size="small" title="Edit" @click.stop="openForm(row)" />
        <el-button :icon="Delete" circle size="small" title="Delete" @click.stop="remove(row)" />
      </template>
    </el-table-column>
  </el-table>

  <ServerFormDialog v-model="formOpen" :server="editing" />
</template>

<style scoped>
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.name {
  font-weight: 600;
}
.clickable :deep(.el-table__row) {
  cursor: pointer;
}
.detail {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 4px 24px 12px;
  cursor: default;
}
</style>
