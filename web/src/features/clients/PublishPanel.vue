<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import { ElMessage } from 'element-plus'
import { Minus, Plus, Promotion } from '@element-plus/icons-vue'
import { publishSubjectError } from '@/domain/subject'
import type { Header } from '@/domain/types'
import { api } from '@/infrastructure/api'
import { errorMessage } from '@/infrastructure/http'

const props = defineProps<{ serverId: number }>()

const msg = reactive<{ subject: string; payload: string; headers: Header[] }>({ subject: '', payload: '', headers: [] })
const sending = ref(false)
const subjectError = computed(() => (msg.subject ? publishSubjectError(msg.subject) : null))

async function send() {
  sending.value = true
  try {
    await api.publish(props.serverId, msg)
    ElMessage.success(`Published to ${msg.subject.trim()}`)
  } catch (e) {
    ElMessage.error(errorMessage(e))
  } finally {
    sending.value = false
  }
}
</script>

<template>
  <el-form label-width="80px" size="small" @submit.prevent="send">
    <el-form-item label="Subject" :error="subjectError ?? undefined">
      <el-input v-model="msg.subject" placeholder="orders.created" class="mono" />
    </el-form-item>
    <el-form-item label="Headers">
      <div class="headers">
        <div v-for="(h, i) in msg.headers" :key="i" class="header-row">
          <el-input v-model="h.name" placeholder="Name" />
          <el-input v-model="h.value" placeholder="Value" />
          <el-button :icon="Minus" circle title="Remove header" @click="msg.headers.splice(i, 1)" />
        </div>
        <el-button :icon="Plus" @click="msg.headers.push({ name: '', value: '' })">Add header</el-button>
      </div>
    </el-form-item>
    <el-form-item label="Payload">
      <el-input v-model="msg.payload" type="textarea" :rows="3" class="mono" placeholder="{&quot;hello&quot;: &quot;world&quot;}" />
    </el-form-item>
    <el-form-item>
      <el-button type="primary" :icon="Promotion" :loading="sending" :disabled="!msg.subject || !!subjectError" @click="send">
        Publish
      </el-button>
      <span class="muted hint">Sent as this server's configured user, so that user's NATS permissions apply.</span>
    </el-form-item>
  </el-form>
</template>

<style scoped>
.headers {
  width: 100%;
}
.header-row {
  display: flex;
  gap: 6px;
  margin-bottom: 4px;
}
.hint {
  margin-left: 8px;
}
</style>
