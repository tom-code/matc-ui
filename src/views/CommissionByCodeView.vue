<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { useDevicesStore } from '../stores/devices'
import type { DeviceDto } from '../types'

const router = useRouter()
const devicesStore = useDevicesStore()

const form = ref({ pairing_code: '', name: '', node_id: 300 })
const loading = ref(false)
const error = ref<string | null>(null)
const success = ref<DeviceDto | null>(null)

async function submit() {
  loading.value = true
  error.value = null
  success.value = null
  try {
    const device = await invoke<DeviceDto>('commission_by_code', {
      pairingCode: form.value.pairing_code.trim(),
      nodeId: form.value.node_id,
      name: form.value.name.trim(),
    })
    success.value = device
    await devicesStore.fetchDevices()
  } catch (e: any) {
    error.value = e?.toString() ?? 'Unknown error'
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="view-container">
    <n-h2>Commission by Pairing Code</n-h2>
    <n-card style="max-width: 520px">
      <n-form label-placement="top" @submit.prevent="submit">
        <n-form-item label="Pairing Code" required>
          <n-input
            v-model:value="form.pairing_code"
            placeholder="e.g. 0251-520-0076"
            :disabled="loading"
          />
        </n-form-item>
        <n-form-item label="Device Name" required>
          <n-input
            v-model:value="form.name"
            placeholder="e.g. kitchen light"
            :disabled="loading"
          />
        </n-form-item>
        <n-form-item label="Node ID">
          <n-input-number
            v-model:value="form.node_id"
            :min="1"
            style="width: 100%"
            :disabled="loading"
          />
        </n-form-item>

        <n-text depth="3" style="font-size: 12px; display: block; margin-bottom: 16px">
          The device will be discovered automatically on the network using the pairing code discriminator.
          Make sure the device is in commissioning mode.
        </n-text>

        <n-button
          type="primary"
          attr-type="submit"
          :loading="loading"
          :disabled="!form.pairing_code || !form.name"
          block
        >
          {{ loading ? 'Commissioning...' : 'Commission Device' }}
        </n-button>
      </n-form>

      <n-alert
        v-if="error"
        type="error"
        :title="error"
        style="margin-top: 16px"
      />

      <n-result
        v-if="success"
        status="success"
        title="Device commissioned!"
        :description="`'${success.name}' (node ${success.node_id}) is ready.`"
        style="margin-top: 16px"
      >
        <template #footer>
          <n-button type="primary" @click="router.push('/devices')">View Devices</n-button>
        </template>
      </n-result>
    </n-card>
  </div>
</template>
