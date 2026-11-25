<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { Refresh, Search } from '@element-plus/icons-vue'
import { num } from '@/domain/format'
import { api } from '@/infrastructure/api'
import { useAsync } from '@/composables/useAsync'

const props = defineProps<{ serverId: number }>()
const subsz = useAsync(api.subsz)
const load = () => subsz.run(props.serverId)
onMounted(load)

const filter = ref('')
const rows = computed(() => {
  const list = subsz.data.value?.subscriptions_list ?? []
  const f = filter.value.trim()
  return f ? list.filter((s) => s.subject.includes(f)) : list
})
</script>

<template>
  <div class="toolbar">
    <el-button size="small" :icon="Refresh" :loading="subsz.loading.value" @click="load">Refresh</el-button>
    <el-input v-model="filter" size="small" placeholder="Filter by subject" clearable :prefix-icon="Search" class="filter" />
    <span v-if="subsz.data.value" class="muted">{{ subsz.data.value.num_subscriptions ?? 0 }} subscriptions</span>
  </div>
  <el-alert v-if="subsz.error.value" type="error" :title="subsz.error.value" :closable="false" />
  <el-table :data="rows" size="small" max-height="600" empty-text="No subscriptions">
    <el-table-column label="Subject" min-width="240"><template #default="{ row }"><span class="mono">{{ row.subject }}</span></template></el-table-column>
    <el-table-column prop="qgroup" label="Queue group" width="140" />
    <el-table-column prop="account" label="Account" width="120" />
    <el-table-column label="Messages" width="100" align="right"><template #default="{ row }">{{ num(row.msgs) }}</template></el-table-column>
    <el-table-column prop="cid" label="Connection" width="100" align="right" />
  </el-table>
</template>

<style scoped>
.toolbar {
  display: flex;
  gap: 8px;
  align-items: center;
  margin-bottom: 8px;
}
.filter {
  width: 240px;
}
</style>
