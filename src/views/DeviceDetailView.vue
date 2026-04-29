<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import type { EndpointTree } from '../types'
import AttributeTree from '../components/AttributeTree.vue'
import CommandPanel from '../components/CommandPanel.vue'
import { useDevicesStore } from '../stores/devices'

const props = defineProps<{ nodeId: string }>()
const router = useRouter()
const store = useDevicesStore()

const nodeId = Number(props.nodeId)

const device = computed(() => store.devices.find(d => d.node_id === nodeId))
const deviceName = computed(() => device.value?.name ?? `Device ${nodeId}`)
const loading = ref(false)
const error = ref<string | null>(null)
const tree = ref<EndpointTree | null>(null)

async function loadTree() {
  loading.value = true
  error.value = null
  try {
    tree.value = await invoke<EndpointTree>('read_attribute_tree', { nodeId })
  } catch (e: any) {
    error.value = e?.toString() ?? 'Unknown error'
  } finally {
    loading.value = false
  }
}

const friendlyError = computed(() => {
  if (!error.value) return null
  const e = error.value.toLowerCase()
  if (e.includes('timed out') || e.includes('timeout'))
    return 'Device not responding (connection timed out)'
  if (e.includes('connect') || e.includes('unreachable') || e.includes('reconnect'))
    return 'Cannot connect to device'
  return error.value
})

onMounted(async () => {
  if (store.devices.length === 0) await store.fetchDevices()
  loadTree()
})
</script>

<template>
  <div class="view-container">
    <div class="view-header">
      <n-button quaternary @click="router.push('/devices')" style="margin-right: 8px">
        &lt;- Back
      </n-button>
      <n-h2 style="margin: 0">{{ deviceName }}</n-h2>
      <n-tag size="small" :bordered="false">Node {{ nodeId }}</n-tag>
      <n-tag v-if="error && !loading" type="error" size="small">Unreachable</n-tag>
      <n-button @click="loadTree" :loading="loading" size="small">Refresh</n-button>
    </div>

    <n-alert v-if="error" type="error" :title="friendlyError" style="margin-bottom: 16px">
      <n-button size="small" type="error" ghost @click="loadTree" :loading="loading"
        style="margin-top: 8px">
        Retry Connection
      </n-button>
    </n-alert>

    <div class="detail-layout">
      <div class="attr-panel">
        <n-card title="Attribute Tree" :segmented="{ content: true }">
          <n-spin :show="loading">
            <div v-if="!loading && !tree && !error" class="hint">
              Click Refresh to load attributes.
            </div>
            <div v-if="!loading && !tree && error" class="hint">
              Attribute data unavailable - device did not respond.
            </div>
            <AttributeTree v-if="tree" :tree="tree" />
          </n-spin>
        </n-card>
      </div>
      <div class="cmd-panel">
        <n-card title="Send Command" :segmented="{ content: true }">
          <CommandPanel :node-id="nodeId" :tree="tree" />
        </n-card>
      </div>
    </div>
  </div>
</template>

<style scoped>
.view-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 20px;
}

.detail-layout {
  display: grid;
  grid-template-columns: 1fr 400px;
  gap: 16px;
  align-items: start;
}

.hint {
  color: var(--n-text-color-disabled);
  padding: 20px;
  text-align: center;
}

@media (max-width: 900px) {
  .detail-layout {
    grid-template-columns: 1fr;
  }
}
</style>
