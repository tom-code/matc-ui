<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useMessage } from 'naive-ui'
import { invoke, Channel } from '@tauri-apps/api/core'
import type { EndpointTree, AttrProgressEvent, OpenCommissioningWindowResult } from '../types'
import AttributeTree from '../components/AttributeTree.vue'
import CommandPanel from '../components/CommandPanel.vue'
import WritePanel from '../components/WritePanel.vue'
import DeviceTreePanel from '../components/DeviceTreePanel.vue'
import DeviceInfoPanel from '../components/DeviceInfoPanel.vue'
import { useDevicesStore } from '../stores/devices'
import { hasAggregator } from '../utils/deviceTree'

const props = defineProps<{ nodeId: string }>()
const router = useRouter()
const message = useMessage()
const store = useDevicesStore()

const nodeId = Number(props.nodeId)

const device = computed(() => store.devices.find(d => d.node_id === nodeId))
const deviceName = computed(() => device.value?.name ?? `Device ${nodeId}`)

const renameMode = ref(false)
const newName = ref('')

function startRename() {
  newName.value = deviceName.value
  renameMode.value = true
}

async function confirmRename() {
  if (newName.value.trim()) {
    await store.renameDevice(nodeId, newName.value.trim())
  }
  renameMode.value = false
}

async function handleRemove() {
  await store.removeDevice(nodeId)
  router.push('/devices')
}

const openCwMode = ref(false)
const openCwBusy = ref(false)
const openCwResultMode = ref(false)
const openCwResult = ref<OpenCommissioningWindowResult | null>(null)
const cwPin = ref(0)
const cwDiscriminator = ref(0)
const cwIterations = ref(2000)
const cwTimeout = ref(180)

function initOpenCommissioningForm() {
  cwPin.value = Math.floor(Math.random() * 99_999_998) + 1
  cwDiscriminator.value = Math.floor(Math.random() * 4096)
  cwIterations.value = 2000
  cwTimeout.value = 180
}

function startOpenCommissioning() {
  initOpenCommissioningForm()
  openCwMode.value = true
}

function cwStatusLabel(status: number): string {
  switch (status) {
    case 0: return 'Success'
    case 2: return 'Busy'
    case 3: return 'PAKE error'
    case 4: return 'Window not open'
    default: return `Status ${status}`
  }
}

async function confirmOpenCommissioning() {
  openCwBusy.value = true
  try {
    const result = await invoke<OpenCommissioningWindowResult>('open_commissioning_window', {
      nodeId,
      pin: cwPin.value,
      discriminator: cwDiscriminator.value,
      iterations: cwIterations.value,
      timeoutSecs: cwTimeout.value,
    })
    openCwResult.value = result
    openCwMode.value = false
    openCwResultMode.value = true
  } catch (e: any) {
    openCwMode.value = false
    message.error(`Open Commissioning Window failed: ${e?.toString() ?? 'Unknown error'}`)
  } finally {
    openCwBusy.value = false
  }
}

const loading = ref(false)
const error = ref<string | null>(null)
const tree = ref<EndpointTree | null>(null)
const progress = ref<AttrProgressEvent | null>(null)
const structureTree = ref<EndpointTree | null>(null)
const structureLoading = ref(false)

async function loadStructure(forceRefresh = false) {
  if (structureLoading.value) return
  structureLoading.value = true
  try {
    structureTree.value = await invoke<EndpointTree>('read_endpoint_structure', { nodeId, forceRefresh })
  } catch {
    // swallow - Commands tab will show empty dropdowns
  } finally {
    structureLoading.value = false
  }
}

async function loadTree(forceRefresh = false) {
  if (loading.value) return
  loading.value = true
  error.value = null
  progress.value = null
  const channel = new Channel<AttrProgressEvent>()
  channel.onmessage = (msg) => { progress.value = msg }
  try {
    tree.value = await invoke<EndpointTree>('read_attribute_tree', { nodeId, channel, forceRefresh })
  } catch (e: any) {
    error.value = e?.toString() ?? 'Unknown error'
  } finally {
    loading.value = false
  }
}

const deviceTreeData = ref<EndpointTree | null>(null)
const deviceTreeLoading = ref(false)

async function loadDeviceTree(forceRefresh = false) {
  if (deviceTreeLoading.value) return
  deviceTreeLoading.value = true
  try {
    deviceTreeData.value = await invoke<EndpointTree>('read_device_tree', { nodeId, forceRefresh })
  } finally {
    deviceTreeLoading.value = false
  }
}

const activeTab = ref<string>('info')

const isBusy = computed(() =>
  loading.value || deviceTreeLoading.value || store.deviceStatus[nodeId] === 'checking'
)

const commandsTree = computed(() => tree.value ?? structureTree.value)

const showDeviceTree = computed(() =>
  store.deviceInfo[nodeId]?.has_aggregator === true ||
  (tree.value != null && hasAggregator(tree.value))
)

watch(activeTab, (tab) => {
  if (tab === 'attributes' && tree.value == null && !loading.value) {
    loadTree()
  }
  if ((tab === 'commands' || tab === 'write') && tree.value == null && structureTree.value == null && !structureLoading.value) {
    loadStructure()
  }
  if (tab === 'device-tree' && deviceTreeData.value == null && tree.value == null && !deviceTreeLoading.value) {
    loadDeviceTree()
  }
})

async function refresh() {
  if (isBusy.value) return
  if (activeTab.value === 'attributes') {
    tree.value = null
    await loadTree(true)
  } else if (activeTab.value === 'device-tree') {
    deviceTreeData.value = null
    await loadDeviceTree(true)
  } else {
    await store.fetchDeviceInfo(nodeId, true).catch(() => {})
  }
}

onMounted(async () => {
  if (store.devices.length === 0) await store.fetchDevices()
  if (store.deviceStatus[nodeId] !== 'checking') {
    store.fetchDeviceInfo(nodeId).catch(() => {})
  }
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
      <n-tag v-if="store.deviceStatus[nodeId] === 'failed'" type="error" size="small">Unreachable</n-tag>
      <n-button @click="refresh" :loading="isBusy" :disabled="isBusy" size="small">Refresh</n-button>
    </div>

    <n-modal v-model:show="renameMode" preset="dialog" title="Rename Device">
      <n-input v-model:value="newName" @keyup.enter="confirmRename" />
      <template #action>
        <n-space>
          <n-button @click="renameMode = false">Cancel</n-button>
          <n-button type="primary" @click="confirmRename">Save</n-button>
        </n-space>
      </template>
    </n-modal>

    <n-modal v-model:show="openCwMode" preset="dialog" title="Open Commissioning Window">
      <n-form label-placement="left" label-width="120">
        <n-form-item label="PIN">
          <n-input-number v-model:value="cwPin" :min="1" :max="99999998" style="width: 100%" />
        </n-form-item>
        <n-form-item label="Discriminator">
          <n-input-number v-model:value="cwDiscriminator" :min="0" :max="4095" style="width: 100%" />
        </n-form-item>
        <n-form-item label="Iterations">
          <n-input-number v-model:value="cwIterations" :min="1000" :max="100000" style="width: 100%" />
        </n-form-item>
        <n-form-item label="Timeout (s)">
          <n-input-number v-model:value="cwTimeout" :min="60" :max="900" style="width: 100%" />
        </n-form-item>
      </n-form>
      <template #action>
        <n-space>
          <n-button @click="openCwMode = false">Cancel</n-button>
          <n-button type="primary" :loading="openCwBusy" @click="confirmOpenCommissioning">Open</n-button>
        </n-space>
      </template>
    </n-modal>

    <n-modal v-model:show="openCwResultMode" preset="dialog" title="Commissioning Window Opened">
      <template v-if="openCwResult">
        <div v-if="openCwResult.status === 0">
          <p>Pairing code (valid for {{ openCwResult.timeout_secs }}s):</p>
          <n-input
            readonly
            :value="openCwResult.manual_pairing_code"
            style="font-family: monospace; font-size: 1.2em"
          />
        </div>
        <div v-else>
          <n-tag type="error">{{ cwStatusLabel(openCwResult.status) }}</n-tag>
        </div>
      </template>
      <template #action>
        <n-button type="primary" @click="openCwResultMode = false">Close</n-button>
      </template>
    </n-modal>

    <n-tabs type="line" v-model:value="activeTab" animated :pane-style="{ paddingTop: '16px' }">
      <n-tab-pane name="info" tab="Info">
        <DeviceInfoPanel :node-id="nodeId" />
        <div class="info-actions">
          <n-button size="small" @click="startRename">Rename</n-button>
          <n-button size="small" @click="startOpenCommissioning">Open Commissioning Window</n-button>
          <n-popconfirm @positive-click="handleRemove">
            <template #trigger>
              <n-button size="small" type="error" ghost>Remove</n-button>
            </template>
            Remove '{{ deviceName }}' from registry?
          </n-popconfirm>
        </div>
      </n-tab-pane>
      <n-tab-pane name="attributes" tab="Attributes">
        <div v-if="loading && progress" class="progress-block">
          <template v-if="progress.phase === 'connecting' || progress.phase === 'discover'">
            <div class="progress-label">
              <span v-if="progress.phase === 'connecting'">Connecting...</span>
              <span v-else-if="progress.endpointCount === 0">Discovering endpoints...</span>
              <span v-else>Discovering: endpoint {{ progress.endpointIndex + 1 }} / {{ progress.endpointCount }}</span>
            </div>
            <n-progress
              type="line"
              :percentage="progress.endpointCount > 0 ? Math.round((progress.endpointIndex + 1) / progress.endpointCount * 100) : 0"
              :show-indicator="false"
            />
          </template>
          <template v-else-if="progress.phase === 'read'">
            <div class="progress-label">
              <span v-if="progress.endpointAttrTotal === 0">Reading...</span>
              <span v-else>{{ progress.currentCluster ?? 'Reading' }} &mdash; {{ progress.endpointAttrIndex }} / {{ progress.endpointAttrTotal }}</span>
            </div>
            <n-progress
              type="line"
              :percentage="progress.endpointAttrTotal > 0 ? Math.round(progress.endpointAttrIndex / progress.endpointAttrTotal * 100) : 0"
              :show-indicator="false"
            />
          </template>
        </div>
        <n-spin v-else-if="loading" :show="true" />
        <div v-if="!loading && !tree && !error" class="hint">
          Click Refresh to load attributes.
        </div>
        <div v-if="!loading && !tree && error" class="hint">
          Attribute data unavailable - device did not respond.
        </div>
        <AttributeTree v-if="tree" :tree="tree" />
      </n-tab-pane>
      <n-tab-pane name="commands" tab="Send Command">
        <CommandPanel :node-id="nodeId" :tree="commandsTree" />
      </n-tab-pane>
      <n-tab-pane name="write" tab="Write">
        <WritePanel :node-id="nodeId" :tree="commandsTree" />
      </n-tab-pane>
      <n-tab-pane v-if="showDeviceTree" name="device-tree" tab="Device Tree">
        <DeviceTreePanel v-if="tree ?? deviceTreeData" :tree="(tree ?? deviceTreeData)!" />
        <n-spin v-else-if="deviceTreeLoading" :show="true" />
        <div v-else class="hint">Loading device tree...</div>
      </n-tab-pane>
    </n-tabs>
  </div>
</template>

<style scoped>
.view-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 20px;
}

.hint {
  color: var(--n-text-color-disabled);
  padding: 20px;
  text-align: center;
}

.progress-block {
  padding: 20px 0;
  max-width: 480px;
}

.progress-label {
  font-size: 13px;
  color: var(--n-text-color-3);
  margin-bottom: 4px;
}

.info-actions {
  display: flex;
  gap: 8px;
  margin-top: 20px;
}
</style>
