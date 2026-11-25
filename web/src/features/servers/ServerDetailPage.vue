<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { useConfigStore } from '@/stores/config'
import { useStatusStore } from '@/stores/status'
import StatusTags from '@/shared/ui/StatusTags.vue'
import JetStreamTab from './tabs/JetStreamTab.vue'
import OverviewTab from './tabs/OverviewTab.vue'
import SubscriptionsTab from './tabs/SubscriptionsTab.vue'

const props = defineProps<{ id: number; tab: string }>()
const router = useRouter()
const config = useConfigStore()
const status = useStatusStore()

const server = computed(() => config.serverById.get(props.id))
// The tab lives in the URL, so a link opens the same tab.
const activeTab = computed({
  get: () => props.tab,
  set: (t: string) => router.replace(`/servers/${props.id}/${t}`),
})
</script>

<template>
  <template v-if="server">
    <div class="page-header">
      <div>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/servers' }">Servers</el-breadcrumb-item>
          <el-breadcrumb-item>{{ server.name }}</el-breadcrumb-item>
        </el-breadcrumb>
        <h2>{{ server.name }} <span class="mono muted">{{ server.host }}:{{ server.port }}</span></h2>
      </div>
      <StatusTags :status="status.of(server.id)" />
    </div>
    <!-- Keyed by server so switching servers resets chart history and loaded data. -->
    <el-tabs :key="server.id" v-model="activeTab">
      <el-tab-pane label="Overview" name="overview" lazy><OverviewTab :server="server" /></el-tab-pane>
      <el-tab-pane label="JetStream" name="jetstream" lazy><JetStreamTab :server-id="server.id" /></el-tab-pane>
      <el-tab-pane label="Subscriptions" name="subscriptions" lazy><SubscriptionsTab :server-id="server.id" /></el-tab-pane>
    </el-tabs>
  </template>
  <el-empty v-else-if="config.loaded" description="This server doesn't exist (it may have been deleted).">
    <el-button @click="router.push('/servers')">Back to servers</el-button>
  </el-empty>
</template>

<style scoped>
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
  padding-top: 16px;
}
h2 {
  margin: 8px 0;
}
</style>
