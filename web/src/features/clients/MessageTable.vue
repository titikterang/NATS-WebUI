<script setup lang="ts">
import { bytes } from '@/domain/format'
import type { MessageRow } from '@/domain/message'

defineProps<{ rows: MessageRow[] }>()
</script>

<template>
  <el-table :data="rows" row-key="key" size="small" height="100%" empty-text="No messages yet. Subscribe to start receiving.">
    <el-table-column type="expand" width="30">
      <template #default="{ row }">
        <div class="detail">
          <div v-if="row.reply" class="muted">Reply to <span class="mono">{{ row.reply }}</span></div>
          <template v-if="row.headers.length">
            <div class="label">Headers</div>
            <table class="headers">
              <tr v-for="([k, v], i) in row.headers" :key="i"><td class="mono k">{{ k }}</td><td class="mono">{{ v }}</td></tr>
            </table>
          </template>
          <div class="label">
            Payload
            <span class="muted">({{ bytes(row.size) }}{{ row.truncated ? ', truncated' : '' }}{{ row.isJson ? ', JSON' : '' }})</span>
          </div>
          <pre class="payload">{{ row.pretty }}</pre>
        </div>
      </template>
    </el-table-column>
    <el-table-column label="Time" width="120"><template #default="{ row }"><span class="mono">{{ row.time }}</span></template></el-table-column>
    <el-table-column label="Subject" min-width="180"><template #default="{ row }"><span class="mono">{{ row.subject }}</span></template></el-table-column>
    <el-table-column label="Headers" width="75" align="center"><template #default="{ row }">{{ row.headers.length || '' }}</template></el-table-column>
    <el-table-column label="Size" width="80" align="right"><template #default="{ row }">{{ bytes(row.size) }}</template></el-table-column>
    <el-table-column label="Payload" min-width="300" show-overflow-tooltip>
      <template #default="{ row }"><span class="mono">{{ row.preview }}</span></template>
    </el-table-column>
  </el-table>
</template>

<style scoped>
.detail {
  padding: 0 16px;
}
.label {
  font-size: 11px;
  color: var(--el-text-color-secondary);
  margin: 6px 0 2px;
}
.headers td {
  padding: 1px 12px 1px 0;
}
.headers .k {
  font-weight: 600;
}
.payload {
  background: var(--el-fill-color-light);
  padding: 8px;
  border-radius: 4px;
  max-height: 320px;
  overflow: auto;
  margin: 0;
  font-size: 12px;
}
</style>
