<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import { Delete, Search, VideoPause, VideoPlay } from '@element-plus/icons-vue'
import { matchesFilter } from '@/domain/message'
import { connView } from '@/domain/status'
import { subjectError } from '@/domain/subject'
import type { Client } from '@/domain/types'
import { api } from '@/infrastructure/api'
import { errorMessage } from '@/infrastructure/http'
import { useClientStream } from '@/composables/useClientStream'
import { useConfigStore } from '@/stores/config'
import MessageTable from './MessageTable.vue'
import PublishPanel from './PublishPanel.vue'

const props = defineProps<{ id: number }>()
const router = useRouter()
const config = useConfigStore()
const stream = useClientStream()

const client = computed(() => config.clientById.get(props.id))
const draft = ref<Client | null>(null)
const suggestions = ref<string[]>([])
const filter = ref('')

// Editing a different client: stop the old stream and start from a fresh draft.
watch(
  () => props.id,
  () => {
    stream.stop()
    stream.clear()
    suggestions.value = []
  },
)
watch(client, (c) => (draft.value = c ? { ...c, subjects: [...c.subjects] } : null), { immediate: true })

const shown = computed(() => stream.messages.value.filter((m) => matchesFilter(m, filter.value)))
const badge = computed(() => (stream.running.value ? connView(stream.status.value) : null))

async function save() {
  if (!draft.value) return
  const bad = draft.value.subjects.map(subjectError).find((e) => e)
  if (bad) {
    ElMessage.error(bad)
    return
  }
  try {
    await config.saveClient(draft.value)
    if (stream.running.value) stream.start(props.id) // apply the new subjects
  } catch (e) {
    ElMessage.error(errorMessage(e))
  }
}

async function loadSuggestions() {
  if (suggestions.value.length || !draft.value) return
  try {
    const subsz = await api.subsz(draft.value.server_id)
    const subjects = (subsz.subscriptions_list ?? []).map((s) => s.subject).filter((s) => !s.startsWith('_INBOX'))
    suggestions.value = [...new Set(subjects)].sort()
  } catch {
    // suggestions are optional
  }
}
</script>

<template>
  <div v-if="draft" class="page">
    <el-breadcrumb separator="/" class="crumbs">
      <el-breadcrumb-item :to="{ path: '/clients' }">Clients</el-breadcrumb-item>
      <el-breadcrumb-item>{{ draft.name }}</el-breadcrumb-item>
    </el-breadcrumb>

    <el-form inline size="small" class="setup" @submit.prevent>
      <el-form-item label="Name"><el-input v-model="draft.name" class="w-name" @change="save" /></el-form-item>
      <el-form-item label="Server">
        <el-select v-model="draft.server_id" class="w-server" @change="() => { suggestions = []; save() }">
          <el-option v-for="s in config.servers" :key="s.id" :label="`${s.name} (${s.host})`" :value="s.id" />
        </el-select>
      </el-form-item>
      <el-form-item label="Subjects">
        <el-select
          v-model="draft.subjects"
          multiple
          filterable
          allow-create
          default-first-option
          placeholder="e.g. orders.>  or  events.*.created"
          class="w-subjects"
          @change="save"
          @focus="loadSuggestions"
        >
          <el-option v-for="s in suggestions" :key="s" :label="s" :value="s" />
        </el-select>
      </el-form-item>
      <el-form-item>
        <el-button v-if="!stream.running.value" type="primary" :icon="VideoPlay" :disabled="!draft.subjects.length" @click="stream.start(id)">
          Subscribe
        </el-button>
        <el-button v-else type="danger" plain :icon="VideoPause" @click="stream.stop()">Stop</el-button>
      </el-form-item>
      <el-form-item v-if="badge">
        <el-tooltip :disabled="!badge.error" :content="badge.error ?? ''">
          <el-tag :type="badge.type" effect="dark">{{ badge.label }}</el-tag>
        </el-tooltip>
      </el-form-item>
    </el-form>

    <el-alert
      v-for="(e, i) in stream.errors.value"
      :key="e + i"
      :title="e"
      type="error"
      show-icon
      class="alert"
      @close="stream.dismissError(i)"
    />

    <div class="toolbar">
      <el-input v-model="filter" size="small" placeholder="Filter by subject or payload" clearable :prefix-icon="Search" class="w-filter" />
      <el-button size="small" :icon="stream.paused.value ? VideoPlay : VideoPause" @click="stream.paused.value = !stream.paused.value">
        {{ stream.paused.value ? 'Resume' : 'Pause' }}
      </el-button>
      <el-button size="small" :icon="Delete" @click="stream.clear()">Clear</el-button>
      <span class="muted">{{ shown.length }} shown · {{ stream.received.value }} received (keeps the latest 500)</span>
    </div>

    <div class="messages"><MessageTable :rows="shown" /></div>

    <el-collapse class="publish">
      <el-collapse-item title="Publish a message" name="publish">
        <PublishPanel :server-id="draft.server_id" />
      </el-collapse-item>
    </el-collapse>
  </div>
  <el-empty v-else-if="config.loaded" description="This client doesn't exist (it may have been deleted).">
    <el-button @click="router.push('/clients')">Back to clients</el-button>
  </el-empty>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  height: 100%;
}
.crumbs {
  padding: 16px 0 8px;
}
.setup {
  border-bottom: 1px solid var(--el-border-color-lighter);
}
.w-name { width: 160px; }
.w-server { width: 220px; }
.w-subjects { width: 360px; }
.w-filter { width: 260px; }
.toolbar {
  display: flex;
  gap: 8px;
  align-items: center;
  margin: 8px 0;
}
.alert {
  margin-bottom: 6px;
}
.messages {
  flex: 1 1 auto;
  min-height: 240px;
}
.publish {
  margin-top: 8px;
}
</style>
