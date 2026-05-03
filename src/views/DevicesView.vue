<script setup lang="ts">
import { h, ref, computed, watch, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { NButton, NTag, NTooltip, NSpin, NSpace } from 'naive-ui'
import type { DataTableColumns } from 'naive-ui'
import { useDevicesStore } from '../stores/devices'
import DeviceCard from '../components/DeviceCard.vue'
import { statusTagType, statusLabel } from '../utils/deviceStatus'
import type { DeviceDto } from '../types'

type ViewMode = 'cards' | 'table'

const store = useDevicesStore()
const router = useRouter()

const viewMode = ref<ViewMode>(
  (localStorage.getItem('devices.viewMode') as ViewMode) || 'cards'
)
watch(viewMode, v => localStorage.setItem('devices.viewMode', v))

onMounted(async () => {
  await store.fetchDevices()
  store.probeAllDevices()
})

async function handleRefresh() {
  await store.fetchDevices()
  store.probeAllDevices()
}

function handleProbe(nodeId: number) {
  store.fetchDeviceInfo(nodeId).catch(() => undefined)
}

const columns = computed<DataTableColumns<DeviceDto>>(() => [
  {
    title: 'Name',
    key: 'name',
    sorter: (a, b) => a.name.localeCompare(b.name),
  },
  {
    title: 'Status',
    key: 'status',
    width: 130,
    render(row) {
      const s = store.deviceStatus[row.node_id]
      const err = store.statusError[row.node_id]
      const tag = h(NTag, { size: 'small', bordered: false, type: statusTagType(s) }, {
        default: () => statusLabel(s),
        ...(s === 'checking' ? { icon: () => h(NSpin, { size: 12 }) } : {}),
      })
      if (s === 'failed' && err) {
        return h(NTooltip, { trigger: 'hover' }, {
          trigger: () => tag,
          default: () => err,
        })
      }
      return tag
    },
  },
  {
    title: 'Actions',
    key: 'actions',
    width: 170,
    render(row) {
      const checking = store.deviceStatus[row.node_id] === 'checking'
      return h(NSpace, { size: 'small', wrap: false }, {
        default: () => [
          h(NButton, {
            size: 'small',
            type: 'primary',
            onClick: () => router.push(`/devices/${row.node_id}`),
          }, { default: () => 'View Details' }),
          h(NButton, {
            size: 'small',
            loading: checking,
            disabled: checking,
            onClick: () => handleProbe(row.node_id),
          }, { default: () => 'Check' }),
        ],
      })
    },
  },
])
</script>

<template>
  <div class="view-container">
    <div class="view-header">
      <n-h2 style="margin: 0">Commissioned Devices</n-h2>
      <n-space align="center">
        <n-radio-group v-model:value="viewMode" size="small">
          <n-radio-button value="cards">Cards</n-radio-button>
          <n-radio-button value="table">Table</n-radio-button>
        </n-radio-group>
        <n-button @click="handleRefresh" :loading="store.loading" size="small">
          Refresh
        </n-button>
      </n-space>
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

      <div v-if="viewMode === 'cards'" class="device-grid">
        <DeviceCard
          v-for="device in store.devices"
          :key="device.node_id"
          :device="device"
          :info="store.deviceInfo[device.node_id]"
          :status="store.deviceStatus[device.node_id]"
          :status-error="store.statusError[device.node_id]"
          @rename="(name) => store.renameDevice(device.node_id, name)"
          @remove="store.removeDevice(device.node_id)"
          @probe="handleProbe(device.node_id)"
        />
      </div>

      <n-data-table
        v-else
        :columns="columns"
        :data="store.devices"
        :row-key="(d: DeviceDto) => d.node_id"
        size="small"
      />
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
