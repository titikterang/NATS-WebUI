<script setup lang="ts">
import { onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { useConfigStore } from '@/stores/config'
import { useStatusStore } from '@/stores/status'
import { errorMessage } from '@/infrastructure/http'
import Sidebar from './layout/Sidebar.vue'

const config = useConfigStore()
const status = useStatusStore()

onMounted(async () => {
  status.start()
  try {
    await config.refresh()
  } catch (e) {
    ElMessage.error(`Couldn't load the configuration: ${errorMessage(e)}`)
  }
})
</script>

<template>
  <el-container class="app">
    <Sidebar />
    <el-main class="content">
      <router-view />
    </el-main>
  </el-container>
</template>

<style>
body {
  margin: 0;
  font-family: 'Lato', system-ui, sans-serif;
  color: #2c3e50;
}
h1, h2, h3, h4 {
  font-family: 'Raleway', system-ui, sans-serif;
}
.mono {
  font-family: 'Roboto Mono', ui-monospace, monospace;
  font-size: 12px;
}
.muted {
  color: var(--el-text-color-secondary);
  font-size: 12px;
}
</style>

<style scoped>
.app {
  height: 100vh;
}
.content {
  padding: 0 20px 20px;
  overflow: auto;
}
</style>
