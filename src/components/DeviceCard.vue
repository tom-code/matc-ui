<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import type { DeviceDto, DeviceInfoDto, DeviceConnectionStatus } from '../types'
import { statusTagType, statusLabel } from '../utils/deviceStatus'

const props = defineProps<{
  device: DeviceDto
  info?: DeviceInfoDto
  status?: DeviceConnectionStatus
  statusError?: string
}>()

const emit = defineEmits<{
  (e: 'probe'): void
}>()

const router = useRouter()

const checking = computed(() => props.status === 'checking')
</script>

<template>
  <n-card :title="device.name" hoverable>
    <template #header-extra>
      <n-space size="small" align="center">
        <span :title="status === 'failed' && statusError ? statusError : undefined">
          <n-tag size="small" :bordered="false" :type="statusTagType(status)">
            <template v-if="checking" #icon>
              <n-spin :size="12" />
            </template>
            {{ statusLabel(status) }}
          </n-tag>
        </span>
        <n-tag size="small" :bordered="false" type="default">
          Node {{ device.node_id }}
        </n-tag>
      </n-space>
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
        <n-button size="small" :loading="checking" :disabled="checking" @click="emit('probe')">
          Check
        </n-button>
      </n-space>
    </template>
  </n-card>

</template>

<style scoped>
.card-body {
  min-height: 60px;
}
</style>
