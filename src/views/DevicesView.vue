<script setup lang="ts">
import { onMounted } from 'vue'
import { useMessage } from 'naive-ui'
import { useDevicesStore } from '../stores/devices'
import DeviceCard from '../components/DeviceCard.vue'

const store = useDevicesStore()
const message = useMessage()

onMounted(() => store.fetchDevices())

async function handleReadInfo(nodeId: number) {
  try {
    await store.fetchDeviceInfo(nodeId)
  } catch (e) {
    message.error(`Failed to read device info: ${e}`)
  }
}
</script>

<template>
  <div class="view-container">
    <div class="view-header">
      <n-h2 style="margin: 0">Commissioned Devices</n-h2>
      <n-button @click="store.fetchDevices()" :loading="store.loading" size="small">
        Refresh
      </n-button>
    </div>

    <n-spin :show="store.loading">
      <div v-if="!store.loading && store.devices.length === 0" class="empty-state">
        <n-empty description="No devices commissioned yet">
          <template #extra>
            <n-space>
              <n-button type="primary" tag="a" href="/commission">Commission by Code</n-button>
              <n-button tag="a" href="/discover">Discover Devices</n-button>
            </n-space>
          </template>
        </n-empty>
      </div>
      <div class="device-grid">
        <DeviceCard
          v-for="device in store.devices"
          :key="device.node_id"
          :device="device"
          :info="store.deviceInfo[device.node_id]"
          :loading="!!store.loadingInfo[device.node_id]"
          @read-info="handleReadInfo(device.node_id)"
          @rename="(name) => store.renameDevice(device.node_id, name)"
          @remove="store.removeDevice(device.node_id)"
        />
      </div>
    </n-spin>
  </div>
</template>

<style scoped>
.view-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 20px;
}

.device-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 16px;
}

.empty-state {
  padding: 60px 0;
}
</style>
