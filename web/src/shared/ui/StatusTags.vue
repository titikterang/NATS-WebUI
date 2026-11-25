<script setup lang="ts">
import { computed } from 'vue'
import { monitorView, natsView } from '@/domain/status'
import type { ServerStatus } from '@/domain/types'

// Two separate health signals: the monitoring port (plain HTTP /varz) can be up while the
// authenticated NATS connection fails, for example on a wrong password.
const props = defineProps<{ status: ServerStatus | null }>()
const nats = computed(() => natsView(props.status))
const monitor = computed(() => monitorView(props.status))
</script>

<template>
  <span class="tags">
    <el-tooltip :disabled="!nats.error" :content="nats.error ?? ''" placement="top">
      <el-tag size="small" :type="nats.type" effect="dark">NATS: {{ nats.label }}</el-tag>
    </el-tooltip>
    <el-tooltip :content="monitor.error ?? 'HTTP monitoring endpoint reachable'" placement="top">
      <el-tag size="small" :type="monitor.type" effect="plain">Monitor: {{ monitor.label }}</el-tag>
    </el-tooltip>
  </span>
</template>

<style scoped>
.tags {
  display: inline-flex;
  gap: 4px;
  flex-wrap: wrap;
}
</style>
