<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { EndpointTree, AttributeDto } from '../types'

const props = defineProps<{
  nodeId: number
  tree: EndpointTree | null
}>()

const loading = ref(false)
const writeOk = ref(false)
const error = ref<string | null>(null)

const selectedEndpoint = ref<number | null>(null)
const selectedClusterId = ref<number | null>(null)
const selectedAttrId = ref<number | null>(null)
const attrList = ref<AttributeDto[]>([])
const valueType = ref<'string' | 'integer'>('string')
const stringValue = ref('')
const intValue = ref<number | null>(null)

const typeOptions = [
  { label: 'String', value: 'string' },
  { label: 'Integer', value: 'integer' },
]

const endpointOptions = computed(() => {
  if (!props.tree) return []
  return props.tree.endpoints.map(ep => ({ label: `Endpoint ${ep.id}`, value: ep.id }))
})

watch(() => props.tree, (t) => {
  if (t && t.endpoints.length > 0 && selectedEndpoint.value === null) {
    selectedEndpoint.value = t.endpoints[0].id
  }
})

const clusterOptions = computed(() => {
  if (!props.tree || selectedEndpoint.value === null) return []
  const ep = props.tree.endpoints.find(e => e.id === selectedEndpoint.value)
  if (!ep) return []
  return ep.clusters.map(cl => ({
    label: `${cl.name} (0x${cl.id.toString(16).padStart(4, '0')})`,
    value: cl.id,
  }))
})

watch(selectedEndpoint, () => {
  selectedClusterId.value = null
  selectedAttrId.value = null
  attrList.value = []
  resetResult()
})

watch(selectedClusterId, async (cid) => {
  selectedAttrId.value = null
  attrList.value = []
  resetResult()
  if (cid === null) return
  try {
    attrList.value = await invoke<AttributeDto[]>('list_cluster_attributes', { clusterId: cid })
  } catch {
    attrList.value = []
  }
})

watch(selectedAttrId, () => { resetResult() })
watch(valueType, () => { resetResult() })

function resetResult() {
  writeOk.value = false
  error.value = null
}

const attrOptions = computed(() =>
  attrList.value.map(a => ({
    label: `${a.name} (0x${a.id.toString(16).padStart(4, '0')})`,
    value: a.id,
  }))
)

const canWrite = computed(() =>
  selectedClusterId.value !== null &&
  selectedAttrId.value !== null &&
  (valueType.value === 'string' || intValue.value !== null)
)

async function doWrite() {
  if (!canWrite.value) return
  loading.value = true
  writeOk.value = false
  error.value = null
  try {
    const value = valueType.value === 'string' ? stringValue.value : intValue.value
    await invoke('write_attribute', {
      nodeId: props.nodeId,
      endpoint: selectedEndpoint.value ?? 1,
      cluster: selectedClusterId.value,
      attrId: selectedAttrId.value,
      valueType: valueType.value,
      value,
    })
    writeOk.value = true
  } catch (e: any) {
    error.value = e?.toString() ?? 'Unknown error'
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div>
    <n-space vertical size="small">
      <n-form label-placement="left" label-width="90" size="small">
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
            :disabled="selectedEndpoint === null"
            filterable
          />
        </n-form-item>
        <n-form-item label="Attribute">
          <n-select
            v-model:value="selectedAttrId"
            :options="attrOptions"
            placeholder="Select attribute"
            :disabled="selectedClusterId === null"
            filterable
          />
        </n-form-item>
        <n-form-item label="Type">
          <n-select
            v-model:value="valueType"
            :options="typeOptions"
          />
        </n-form-item>
        <n-form-item label="Value">
          <n-input
            v-if="valueType === 'string'"
            v-model:value="stringValue"
            placeholder="Enter string value"
          />
          <n-input-number
            v-else
            v-model:value="intValue"
            placeholder="Enter integer value"
            style="width: 100%"
          />
        </n-form-item>
      </n-form>

      <n-button
        type="primary"
        block
        :loading="loading"
        :disabled="!canWrite"
        @click="doWrite"
      >
        Write Attribute
      </n-button>

      <n-tag v-if="writeOk" type="success">Write OK</n-tag>
      <n-alert v-if="error" type="error" :title="error" />
    </n-space>
  </div>
</template>
