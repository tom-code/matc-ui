<script setup lang="ts">
import { computed } from 'vue'
import { useDevicesStore } from '../stores/devices'
import { statusTagType, statusLabel } from '../utils/deviceStatus'

const props = defineProps<{ nodeId: number }>()
const store = useDevicesStore()

const info = computed(() => store.deviceInfo[props.nodeId])
const status = computed(() => store.deviceStatus[props.nodeId] ?? 'unknown')
const statusErr = computed(() => store.statusError[props.nodeId])
const device = computed(() => store.devices.find(d => d.node_id === props.nodeId))
const address = computed(() => info.value?.address ?? device.value?.address ?? '')
</script>

<template>
  <div class="info-list">
    <div class="info-row">
      <span class="info-label">Status</span>
      <span class="info-value">
        <span :title="status === 'failed' && statusErr ? statusErr : undefined">
          <n-tag :type="statusTagType(status)" size="small" :bordered="false">{{ statusLabel(status) }}</n-tag>
        </span>
      </span>
    </div>
    <div class="info-row">
      <span class="info-label">Node ID</span>
      <span class="info-value">{{ nodeId }}</span>
    </div>
    <div class="info-row">
      <span class="info-label">Address</span>
      <span class="info-value">{{ address || '-' }}</span>
    </div>
    <div class="info-row">
      <span class="info-label">Vendor</span>
      <span class="info-value">{{ info?.vendor_name || '-' }}</span>
    </div>
    <div class="info-row">
      <span class="info-label">Product</span>
      <span class="info-value">{{ info?.product_name || '-' }}</span>
    </div>
    <div class="info-row">
      <span class="info-label">Hardware version</span>
      <span class="info-value">{{ info?.hw_version || '-' }}</span>
    </div>
    <div class="info-row">
      <span class="info-label">Software version</span>
      <span class="info-value">{{ info?.sw_version || '-' }}</span>
    </div>
  </div>
</template>

<style scoped>
.info-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.info-row {
  display: grid;
  grid-template-columns: 140px 1fr;
  align-items: baseline;
  gap: 8px;
}

.info-label {
  font-size: 13px;
  color: var(--n-text-color-3);
}

.info-value {
  font-size: 14px;
}
</style>
