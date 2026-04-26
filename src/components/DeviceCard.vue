<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import type { DeviceDto, DeviceInfoDto } from '../types'

const props = defineProps<{
  device: DeviceDto
  info?: DeviceInfoDto
  loading?: boolean
}>()

const emit = defineEmits<{
  (e: 'read-info'): void
  (e: 'rename', name: string): void
  (e: 'remove'): void
}>()

const router = useRouter()
const renameMode = ref(false)
const newName = ref('')

function startRename() {
  newName.value = props.device.name
  renameMode.value = true
}

function confirmRename() {
  if (newName.value.trim()) {
    emit('rename', newName.value.trim())
  }
  renameMode.value = false
}
</script>

<template>
  <n-card :title="device.name" hoverable>
    <template #header-extra>
      <n-tag size="small" :bordered="false" type="default">
        Node {{ device.node_id }}
      </n-tag>
    </template>

    <div class="card-body">
      <n-text depth="3" style="font-size: 12px; display: block; margin-bottom: 8px">
        {{ device.address || 'No address stored' }}
      </n-text>

      <template v-if="info">
        <n-descriptions :column="1" size="small" label-placement="left">
          <n-descriptions-item label="Vendor" v-if="info.vendor_name">
            {{ info.vendor_name }}
          </n-descriptions-item>
          <n-descriptions-item label="Product" v-if="info.product_name">
            {{ info.product_name }}
          </n-descriptions-item>
          <n-descriptions-item label="Firmware" v-if="info.sw_version">
            {{ info.sw_version }}
          </n-descriptions-item>
        </n-descriptions>
      </template>
    </div>

    <template #action>
      <n-space wrap>
        <n-button size="small" type="primary" @click="router.push(`/devices/${device.node_id}`)">
          View Details
        </n-button>
        <n-button size="small" :loading="loading" :disabled="loading" @click="emit('read-info')">
          Read Info
        </n-button>
        <n-button size="small" @click="startRename">Rename</n-button>
        <n-popconfirm @positive-click="emit('remove')">
          <template #trigger>
            <n-button size="small" type="error" ghost>Remove</n-button>
          </template>
          Remove '{{ device.name }}' from registry?
        </n-popconfirm>
      </n-space>
    </template>
  </n-card>

  <n-modal v-model:show="renameMode" preset="dialog" title="Rename Device">
    <n-input v-model:value="newName" @keyup.enter="confirmRename" />
    <template #action>
      <n-space>
        <n-button @click="renameMode = false">Cancel</n-button>
        <n-button type="primary" @click="confirmRename">Save</n-button>
      </n-space>
    </template>
  </n-modal>
</template>

<style scoped>
.card-body {
  min-height: 60px;
}
</style>
