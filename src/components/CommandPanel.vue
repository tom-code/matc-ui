<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { EndpointTree, CommandDto, CommandSchemaDto, CommandFieldDto, FieldKind } from '../types'

const props = defineProps<{
  nodeId: number
  tree: EndpointTree | null
}>()

// --- shared ---
const loading = ref(false)
const result = ref<string | null>(null)
const error = ref<string | null>(null)
const activeTab = ref<'friendly' | 'raw'>('friendly')

// --- friendly mode ---
const selectedEndpoint = ref<number | null>(null)
const selectedClusterId = ref<number | null>(null)
const selectedCommandId = ref<number | null>(null)
const commandList = ref<CommandDto[]>([])
const schema = ref<CommandSchemaDto | null>(null)
const fieldValues = ref<Record<string, unknown>>({})
const fieldEnabled = ref<Record<string, boolean>>({})
const schemaLoading = ref(false)

// --- raw mode ---
const rawEndpoint = ref(1)
const rawCluster = ref('')
const rawCommand = ref('')
const rawPayload = ref('')

// Set of cluster IDs on the current endpoint that the codec knows commands for.
// Populated by probing list_cluster_commands for each device cluster.
const clustersWithCommands = ref<Set<number>>(new Set())
let refreshToken = 0

async function refreshClustersWithCommands() {
  const token = ++refreshToken
  if (!props.tree || selectedEndpoint.value === null) {
    clustersWithCommands.value = new Set()
    return
  }
  const ep = props.tree.endpoints.find(e => e.id === selectedEndpoint.value)
  if (!ep) {
    clustersWithCommands.value = new Set()
    return
  }
  try {
    const results = await Promise.all(
      ep.clusters.map(cl => invoke<CommandDto[]>('list_cluster_commands', { clusterId: cl.id }))
    )
    if (token !== refreshToken) return
    const withCmds = new Set<number>()
    ep.clusters.forEach((cl, i) => {
      if (results[i].length > 0) withCmds.add(cl.id)
    })
    clustersWithCommands.value = withCmds
    if (selectedClusterId.value !== null && !withCmds.has(selectedClusterId.value)) {
      selectedClusterId.value = null
    }
  } catch {
    if (token !== refreshToken) return
    clustersWithCommands.value = new Set()
  }
}

// Endpoints from tree
const endpointOptions = computed(() => {
  if (props.tree) {
    return props.tree.endpoints.map(ep => ({ label: `Endpoint ${ep.id}`, value: ep.id }))
  }
  return [{ label: 'Endpoint 1', value: 1 }]
})

// Set default endpoint when tree loads; repopulate cluster filter when tree or endpoint changes
watch(() => props.tree, (t) => {
  if (t && t.endpoints.length > 0 && selectedEndpoint.value === null) {
    selectedEndpoint.value = t.endpoints[0].id
  }
  refreshClustersWithCommands()
})

watch(selectedEndpoint, refreshClustersWithCommands)

// Cluster options: device clusters on selected endpoint that have codec-known commands
const clusterOptions = computed(() => {
  if (!props.tree || selectedEndpoint.value === null) return []
  const ep = props.tree.endpoints.find(e => e.id === selectedEndpoint.value)
  if (!ep) return []
  return ep.clusters
    .filter(cl => clustersWithCommands.value.has(cl.id))
    .map(cl => ({
      label: `${cl.name} (0x${cl.id.toString(16).padStart(4, '0')})`,
      value: cl.id,
    }))
})

// When cluster changes, load command list and reset command / schema
watch(selectedClusterId, async (cid) => {
  selectedCommandId.value = null
  schema.value = null
  fieldValues.value = {}
  fieldEnabled.value = {}
  if (cid === null) {
    commandList.value = []
    return
  }
  try {
    commandList.value = await invoke<CommandDto[]>('list_cluster_commands', { clusterId: cid })
  } catch {
    commandList.value = []
  }
})

// When command changes, load schema
watch(selectedCommandId, async (cmdId) => {
  schema.value = null
  fieldValues.value = {}
  fieldEnabled.value = {}
  if (cmdId === null || selectedClusterId.value === null) return
  schemaLoading.value = true
  try {
    const s = await invoke<CommandSchemaDto | null>('get_command_schema', {
      clusterId: selectedClusterId.value,
      commandId: cmdId,
    })
    schema.value = s
    if (s) {
      // Set defaults
      for (const f of s.fields) {
        fieldEnabled.value[f.name] = !f.optional && !f.nullable
        fieldValues.value[f.name] = defaultValue(f)
      }
    }
  } finally {
    schemaLoading.value = false
  }
})

function defaultValue(f: CommandFieldDto): unknown {
  const k = f.kind
  if (k.type === 'bool') return false
  if (k.type === 'string') return ''
  if (k.type === 'octet_string') return ''
  if (k.type === 'enum') return k.variants[0]?.[0] ?? 0
  if (k.type === 'bitmap') return 0
  return 0
}

function isComplex(k: FieldKind): boolean {
  return k.type === 'struct' || k.type === 'list'
}

const hasComplexFields = computed(() => {
  if (!schema.value) return false
  return schema.value.fields.some(f => isComplex(f.kind))
})

// Build args JSON object from fieldValues + fieldEnabled
function buildArgs(): Record<string, unknown> {
  const args: Record<string, unknown> = {}
  if (!schema.value) return args
  for (const f of schema.value.fields) {
    if (isComplex(f.kind)) continue
    if (!fieldEnabled.value[f.name]) continue
    args[f.name] = fieldValues.value[f.name]
  }
  return args
}

async function sendFriendly() {
  if (selectedClusterId.value === null || selectedCommandId.value === null) return
  loading.value = true
  result.value = null
  error.value = null
  try {
    const ep = selectedEndpoint.value ?? 1
    const args = buildArgs()
    const res = await invoke<string>('invoke_command_typed', {
      nodeId: props.nodeId,
      endpoint: ep,
      cluster: selectedClusterId.value,
      command: selectedCommandId.value,
      args,
    })
    result.value = res
  } catch (e: any) {
    error.value = e?.toString() ?? 'Unknown error'
  } finally {
    loading.value = false
  }
}

// --- raw mode ---
function parseHexOrDec(s: string): number {
  s = s.trim()
  if (s.startsWith('0x') || s.startsWith('0X')) return parseInt(s.slice(2), 16)
  return parseInt(s, 10)
}

async function sendRaw() {
  loading.value = true
  result.value = null
  error.value = null
  try {
    const clusterId = parseHexOrDec(rawCluster.value)
    const commandId = parseHexOrDec(rawCommand.value)
    const res = await invoke<string>('invoke_command', {
      nodeId: props.nodeId,
      endpoint: rawEndpoint.value,
      cluster: clusterId,
      command: commandId,
      payloadHex: rawPayload.value.trim(),
    })
    result.value = res
  } catch (e: any) {
    error.value = e?.toString() ?? 'Unknown error'
  } finally {
    loading.value = false
  }
}

// Quick presets: populate friendly tab
const presets = [
  { label: 'On', endpoint: 1, clusterId: 0x0006, commandId: 0x01 },
  { label: 'Off', endpoint: 1, clusterId: 0x0006, commandId: 0x00 },
  { label: 'Toggle', endpoint: 1, clusterId: 0x0006, commandId: 0x02 },
]

function applyPreset(p: typeof presets[0]) {
  activeTab.value = 'friendly'
  selectedEndpoint.value = p.endpoint
  selectedClusterId.value = p.clusterId
  selectedCommandId.value = p.commandId
}

// Helper: bitmap checkbox model
function bitmapValue(fname: string): number {
  return (fieldValues.value[fname] as number) || 0
}

function bitmapBitChecked(fname: string, bit: number): boolean {
  return (bitmapValue(fname) & bit) !== 0
}

function bitmapToggleBit(fname: string, bit: number, checked: boolean) {
  const cur = bitmapValue(fname)
  fieldValues.value[fname] = checked ? (cur | bit) : (cur & ~bit)
}
</script>

<template>
  <div>
    <n-space vertical size="small">
      <!-- Quick presets -->
      <div>
        <n-text strong style="font-size: 12px">Quick Presets</n-text>
        <n-space size="small" style="margin-top: 4px" wrap>
          <n-button
            v-for="p in presets"
            :key="p.label"
            size="tiny"
            @click="applyPreset(p)"
          >
            {{ p.label }}
          </n-button>
        </n-space>
      </div>

      <n-divider style="margin: 8px 0" />

      <!-- Mode tabs -->
      <n-tabs v-model:value="activeTab" type="line" size="small">
        <!-- ====== FRIENDLY TAB ====== -->
        <n-tab-pane name="friendly" tab="Friendly">
          <n-form label-placement="left" label-width="80" size="small">
            <n-form-item label="Endpoint">
              <n-select
                v-model:value="selectedEndpoint"
                :options="endpointOptions"
                placeholder="Select endpoint"
              />
            </n-form-item>
            <n-form-item label="Cluster">
              <n-select
                v-model:value="selectedClusterId"
                :options="clusterOptions"
                placeholder="Select cluster"
                filterable
              />
            </n-form-item>
            <n-form-item label="Command">
              <n-select
                v-model:value="selectedCommandId"
                :options="commandList.map(c => ({
                  label: `${c.name} (0x${c.id.toString(16).padStart(2,'0')})`,
                  value: c.id,
                }))"
                placeholder="Select command"
                :disabled="!selectedClusterId"
              />
            </n-form-item>
          </n-form>

          <!-- Schema-driven parameter form -->
          <div v-if="schemaLoading" style="text-align:center;padding:8px">
            <n-spin size="small" />
          </div>

          <div v-else-if="schema && schema.fields.length > 0">
            <n-text strong style="font-size: 12px">Parameters</n-text>
            <n-form label-placement="left" label-width="120" size="small" style="margin-top:4px">
              <template v-for="f in schema.fields" :key="f.name">
                <!-- optional/nullable toggle -->
                <n-form-item :label="f.name">
                  <n-space align="center" size="small" style="width:100%">
                    <n-checkbox
                      v-if="f.optional || f.nullable"
                      v-model:checked="fieldEnabled[f.name]"
                      size="small"
                    />

                    <!-- bool -->
                    <n-switch
                      v-if="f.kind.type === 'bool'"
                      v-model:value="(fieldValues[f.name] as boolean)"
                      :disabled="(f.optional || f.nullable) && !fieldEnabled[f.name]"
                    />

                    <!-- string -->
                    <n-input
                      v-else-if="f.kind.type === 'string'"
                      v-model:value="(fieldValues[f.name] as string)"
                      :disabled="(f.optional || f.nullable) && !fieldEnabled[f.name]"
                      style="flex:1"
                    />

                    <!-- octet string -->
                    <n-input
                      v-else-if="f.kind.type === 'octet_string'"
                      v-model:value="(fieldValues[f.name] as string)"
                      placeholder="hex bytes"
                      :disabled="(f.optional || f.nullable) && !fieldEnabled[f.name]"
                      style="flex:1"
                    />

                    <!-- enum -->
                    <n-select
                      v-else-if="f.kind.type === 'enum'"
                      v-model:value="(fieldValues[f.name] as number)"
                      :options="f.kind.variants.map(([v, n]) => ({ label: n, value: v }))"
                      :disabled="(f.optional || f.nullable) && !fieldEnabled[f.name]"
                      style="flex:1"
                    />

                    <!-- bitmap -->
                    <n-space
                      v-else-if="f.kind.type === 'bitmap'"
                      wrap
                      style="flex:1"
                    >
                      <n-checkbox
                        v-for="[bit, bname] in f.kind.bits"
                        :key="bname"
                        :checked="bitmapBitChecked(f.name, bit)"
                        @update:checked="(v: boolean) => bitmapToggleBit(f.name, bit, v)"
                        :disabled="(f.optional || f.nullable) && !fieldEnabled[f.name]"
                        size="small"
                      >{{ bname }}</n-checkbox>
                    </n-space>

                    <!-- struct / list: JSON textarea -->
                    <n-input
                      v-else-if="f.kind.type === 'struct' || f.kind.type === 'list'"
                      v-model:value="(fieldValues[f.name] as string)"
                      type="textarea"
                      placeholder="complex type - use Raw tab"
                      :disabled="true"
                      :rows="2"
                      style="flex:1"
                    />

                    <!-- numeric scalars -->
                    <n-input-number
                      v-else
                      v-model:value="(fieldValues[f.name] as number)"
                      :disabled="(f.optional || f.nullable) && !fieldEnabled[f.name]"
                      style="flex:1"
                    />
                  </n-space>
                </n-form-item>
              </template>
            </n-form>

            <n-alert
              v-if="hasComplexFields"
              type="info"
              size="small"
              style="margin-top:4px"
            >
              Some fields have complex types (struct/list) — use the Raw tab for those.
            </n-alert>
          </div>

          <div v-else-if="schema && schema.fields.length === 0">
            <n-text depth="3" style="font-size:12px">No parameters required.</n-text>
          </div>

          <n-button
            type="primary"
            block
            :loading="loading"
            :disabled="selectedClusterId === null || selectedCommandId === null"
            style="margin-top:8px"
            @click="sendFriendly"
          >
            Send Command
          </n-button>
        </n-tab-pane>

        <!-- ====== RAW TAB ====== -->
        <n-tab-pane name="raw" tab="Raw">
          <n-form label-placement="left" label-width="80" size="small">
            <n-form-item label="Endpoint">
              <n-input-number v-model:value="rawEndpoint" :min="0" style="width:100%" />
            </n-form-item>
            <n-form-item label="Cluster">
              <n-input v-model:value="rawCluster" placeholder="0x0006 or 6" />
            </n-form-item>
            <n-form-item label="Command">
              <n-input v-model:value="rawCommand" placeholder="0x01 or 1" />
            </n-form-item>
            <n-form-item label="Payload (hex)">
              <n-input v-model:value="rawPayload" placeholder="optional hex bytes" />
            </n-form-item>
          </n-form>

          <n-button
            type="primary"
            block
            :loading="loading"
            :disabled="!rawCluster || !rawCommand"
            @click="sendRaw"
          >
            Send Command
          </n-button>
        </n-tab-pane>
      </n-tabs>

      <!-- Shared result / error display -->
      <n-alert v-if="error" type="error" :title="error" />
      <div v-if="result" class="result-box">
        <n-text strong style="font-size: 12px">Response:</n-text>
        <pre class="result-pre">{{ result }}</pre>
      </div>
    </n-space>
  </div>
</template>

<style scoped>
.result-box {
  margin-top: 8px;
}

.result-pre {
  font-size: 11px;
  background: var(--n-code-color);
  border-radius: 4px;
  padding: 8px;
  overflow: auto;
  max-height: 200px;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
