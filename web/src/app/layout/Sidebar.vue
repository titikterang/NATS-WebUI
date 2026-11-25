<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { Monitor, Connection } from '@element-plus/icons-vue'
import { useConfigStore } from '@/stores/config'
import { useStatusStore } from '@/stores/status'
import { natsView } from '@/domain/status'

const route = useRoute()
const config = useConfigStore()
const status = useStatusStore()

// Highlight the section and item for nested routes (e.g. /servers/3/jetstream → /servers/3).
const active = computed(() => {
  const [, section, id] = route.path.split('/')
  return id ? `/${section}/${id}` : `/${section}`
})
</script>

<template>
  <el-aside width="230px" class="aside">
    <router-link to="/servers" class="logo">
      <img src="/large-logo.png" alt="NATS" />
      <span>WEB-UI</span>
    </router-link>
    <el-menu :default-active="active" router class="menu">
      <el-menu-item index="/servers">
        <el-icon><Monitor /></el-icon><span>Servers</span>
      </el-menu-item>
      <el-menu-item v-for="s in config.servers" :key="`s${s.id}`" :index="`/servers/${s.id}`" class="sub">
        <span class="ellipsis">{{ s.name }}</span>
        <el-tooltip :content="`NATS: ${natsView(status.of(s.id)).label}`" placement="right">
          <span class="dot" :class="natsView(status.of(s.id)).type">●</span>
        </el-tooltip>
      </el-menu-item>
      <el-menu-item index="/clients">
        <el-icon><Connection /></el-icon><span>Clients</span>
      </el-menu-item>
      <el-menu-item v-for="c in config.clients" :key="`c${c.id}`" :index="`/clients/${c.id}`" class="sub">
        <span class="ellipsis">{{ c.name }}</span>
      </el-menu-item>
    </el-menu>
    <div v-if="!status.online" class="offline">Live status disconnected. Reconnecting…</div>
  </el-aside>
</template>

<style scoped>
.aside {
  display: flex;
  flex-direction: column;
  border-right: 1px solid var(--el-border-color-light);
  overflow-x: hidden;
}
.logo {
  display: flex;
  align-items: flex-end;
  gap: 6px;
  padding: 20px 16px;
  text-decoration: none;
  color: inherit;
}
.logo img {
  height: 32px;
}
.logo span {
  font-family: 'Lexend Mega', sans-serif;
  font-size: x-small;
  white-space: nowrap;
}
.menu {
  border-right: none;
  flex: 1;
  overflow-y: auto;
}
.sub {
  padding-left: 48px !important;
  height: 40px;
  display: flex;
  justify-content: space-between;
}
.ellipsis {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.dot.success { color: var(--el-color-success); }
.dot.info { color: var(--el-color-info); }
.dot.danger { color: var(--el-color-danger); }
.offline {
  padding: 8px 16px;
  font-size: 12px;
  color: var(--el-color-danger);
}
</style>
