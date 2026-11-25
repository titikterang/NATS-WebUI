<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { Refresh } from '@element-plus/icons-vue'
import { bytes, num, orDash } from '@/domain/format'
import type { StreamInfo } from '@/domain/types'
import { api } from '@/infrastructure/api'
import { useAsync } from '@/composables/useAsync'

const props = defineProps<{ serverId: number }>()
const jsz = useAsync(api.jsz)
const load = () => jsz.run(props.serverId)
onMounted(load)

const streams = computed<(StreamInfo & { account: string })[]>(() =>
  (jsz.data.value?.account_details ?? []).flatMap((acc) =>
    (acc.stream_detail ?? []).map((s) => ({ ...s, account: acc.name })),
  ),
)
</script>

<template>
  <div class="toolbar">
    <el-button size="small" :icon="Refresh" :loading="jsz.loading.value" @click="load">Refresh</el-button>
    <span v-if="jsz.data.value" class="muted">
      {{ jsz.data.value.streams ?? 0 }} streams · {{ jsz.data.value.consumers ?? 0 }} consumers ·
      {{ num(jsz.data.value.messages) }} messages · {{ bytes(jsz.data.value.bytes) }}
    </span>
  </div>
  <el-alert v-if="jsz.error.value" type="error" :title="jsz.error.value" :closable="false" />
  <el-table :data="streams" row-key="name" size="small" empty-text="No JetStream streams on this server">
    <el-table-column type="expand" width="36">
      <template #default="{ row }">
        <el-table :data="row.consumer_detail ?? []" size="small" empty-text="No consumers">
          <el-table-column prop="name" label="Consumer" />
          <el-table-column label="Pending" align="right"><template #default="c">{{ num(c.row.num_pending) }}</template></el-table-column>
          <el-table-column label="Ack pending" align="right"><template #default="c">{{ num(c.row.num_ack_pending) }}</template></el-table-column>
          <el-table-column label="Redelivered" align="right"><template #default="c">{{ num(c.row.num_redelivered) }}</template></el-table-column>
          <el-table-column label="Delivered seq" align="right"><template #default="c">{{ orDash(c.row.delivered?.stream_seq) }}</template></el-table-column>
        </el-table>
      </template>
    </el-table-column>
    <el-table-column prop="name" label="Stream" min-width="140" />
    <el-table-column prop="account" label="Account" width="110" />
    <el-table-column label="Subjects" min-width="160">
      <template #default="{ row }"><span class="mono">{{ (row.config?.subjects ?? []).join(', ') }}</span></template>
    </el-table-column>
    <el-table-column label="Storage" width="80"><template #default="{ row }">{{ orDash(row.config?.storage) }}</template></el-table-column>
    <el-table-column label="Messages" width="95" align="right"><template #default="{ row }">{{ num(row.state.messages) }}</template></el-table-column>
    <el-table-column label="Bytes" width="90" align="right"><template #default="{ row }">{{ bytes(row.state.bytes) }}</template></el-table-column>
    <el-table-column label="Consumers" width="95" align="right"><template #default="{ row }">{{ row.state.consumer_count }}</template></el-table-column>
    <el-table-column label="Last seq" width="90" align="right"><template #default="{ row }">{{ row.state.last_seq }}</template></el-table-column>
  </el-table>
</template>

<style scoped>
.toolbar {
  display: flex;
  gap: 8px;
  align-items: center;
  margin-bottom: 8px;
}
</style>
