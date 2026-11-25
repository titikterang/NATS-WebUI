<script setup lang="ts">
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Delete, Plus } from '@element-plus/icons-vue'
import type { Client } from '@/domain/types'
import { errorMessage } from '@/infrastructure/http'
import { useConfigStore } from '@/stores/config'

const router = useRouter()
const config = useConfigStore()

function serverLabel(id: number) {
  const s = config.serverById.get(id)
  return s ? `${s.name} (${s.host}:${s.port})` : `#${id} (deleted)`
}

async function create() {
  const first = config.servers[0]
  if (!first) return
  try {
    const c = await config.saveClient({ name: 'New client', server_id: first.id, subjects: [] })
    router.push(`/clients/${c.id}`)
  } catch (e) {
    ElMessage.error(errorMessage(e))
  }
}

async function remove(c: Client) {
  try {
    await ElMessageBox.confirm(`Delete client "${c.name}"?`, 'Delete client', { type: 'warning', confirmButtonText: 'Delete' })
  } catch {
    return
  }
  await config.deleteClient(c.id).catch((e) => ElMessage.error(errorMessage(e)))
}
</script>

<template>
  <div class="page-header">
    <h2>Clients</h2>
    <el-tooltip content="Add a server first" :disabled="config.servers.length > 0" placement="left">
      <span><el-button type="primary" :icon="Plus" :disabled="!config.servers.length" @click="create">Add client</el-button></span>
    </el-tooltip>
  </div>
  <el-table :data="config.clients" class="clickable" @row-click="(c: Client) => router.push(`/clients/${c.id}`)">
    <template #empty>No clients yet. A client is a saved set of subjects to watch on one server.</template>
    <el-table-column prop="name" label="Name" min-width="150" />
    <el-table-column label="Server" min-width="180"><template #default="{ row }">{{ serverLabel(row.server_id) }}</template></el-table-column>
    <el-table-column label="Subjects" min-width="240">
      <template #default="{ row }">
        <el-tag v-for="s in row.subjects" :key="s" size="small" class="mono subject">{{ s }}</el-tag>
        <span v-if="!row.subjects.length" class="muted">none</span>
      </template>
    </el-table-column>
    <el-table-column width="60" align="right">
      <template #default="{ row }"><el-button :icon="Delete" circle size="small" title="Delete" @click.stop="remove(row)" /></template>
    </el-table-column>
  </el-table>
</template>

<style scoped>
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.clickable :deep(.el-table__row) {
  cursor: pointer;
}
.subject {
  margin-right: 4px;
}
</style>
