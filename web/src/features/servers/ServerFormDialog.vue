<script setup lang="ts">
import { reactive, ref, watch } from 'vue'
import { ElMessage, type FormInstance, type FormRules } from 'element-plus'
import { hostError } from '@/domain/subject'
import type { Server, ServerInput } from '@/domain/types'
import { errorMessage } from '@/infrastructure/http'
import { useConfigStore } from '@/stores/config'

/** Add or edit a server. `server` null means "add". */
const props = defineProps<{ server: Server | null }>()
const visible = defineModel<boolean>({ required: true })
const emit = defineEmits<{ saved: [Server] }>()

const config = useConfigStore()
const formRef = ref<FormInstance>()
const saving = ref(false)

interface Form {
  name: string
  host: string
  port: number
  monitoring_port: number
  username: string
  password: string
}
const blank = (): Form => ({ name: '', host: '', port: 4222, monitoring_port: 8222, username: '', password: '' })
const form = reactive<Form>(blank())

watch(visible, (open) => {
  if (!open) return
  Object.assign(form, blank(), props.server ? { ...props.server, username: props.server.username ?? '', password: '' } : {})
  formRef.value?.clearValidate()
})

const rules: FormRules<Form> = {
  name: [{ required: true, message: 'Enter a name', trigger: 'blur' }],
  host: [{ validator: (_r, v: string, cb) => { const e = hostError(v); cb(e ? new Error(e) : undefined) }, trigger: 'blur' }],
  port: [{ required: true, message: 'Enter the NATS port' }],
  monitoring_port: [{ required: true, message: 'Enter the monitoring port' }],
}

async function save() {
  if (!(await formRef.value?.validate().catch(() => false))) return
  saving.value = true
  try {
    const input: ServerInput = {
      id: props.server?.id,
      name: form.name,
      host: form.host,
      port: form.port,
      monitoring_port: form.monitoring_port,
      username: form.username.trim() || null,
      // Blank on edit means "keep the stored password".
      password: form.password || null,
    }
    const saved = await config.saveServer(input)
    ElMessage.success(`Saved ${saved.name}`)
    visible.value = false
    emit('saved', saved)
  } catch (e) {
    ElMessage.error(errorMessage(e))
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <el-dialog v-model="visible" :title="server ? `Edit ${server.name}` : 'Add server'" width="480px">
    <el-form ref="formRef" :model="form" :rules="rules" label-width="130px" @submit.prevent="save">
      <el-form-item label="Name" prop="name"><el-input v-model="form.name" placeholder="nats-sandbox" /></el-form-item>
      <el-form-item label="Host" prop="host"><el-input v-model="form.host" placeholder="nats.example.internal" /></el-form-item>
      <el-form-item label="NATS port" prop="port"><el-input-number v-model="form.port" :min="1" :max="65535" :controls="false" /></el-form-item>
      <el-form-item label="Monitoring port" prop="monitoring_port">
        <el-input-number v-model="form.monitoring_port" :min="1" :max="65535" :controls="false" />
      </el-form-item>
      <el-form-item label="Username">
        <el-input v-model="form.username" placeholder="Leave blank if the server has no auth" autocomplete="off" />
      </el-form-item>
      <el-form-item label="Password">
        <el-input
          v-model="form.password"
          type="password"
          show-password
          autocomplete="new-password"
          :disabled="!form.username.trim()"
          :placeholder="server?.has_password ? 'Unchanged (leave blank to keep)' : ''"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="visible = false">Cancel</el-button>
      <el-button type="primary" :loading="saving" @click="save">Save</el-button>
    </template>
  </el-dialog>
</template>
