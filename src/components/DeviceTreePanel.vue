<script setup lang="ts">
import { h, computed, ref, onMounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { NTag, NSpace } from 'naive-ui'
import type { TreeOption } from 'naive-ui'
import type { EndpointTree } from '../types'
import {
  buildDeviceTree,
  primaryDeviceType,
  DEVICE_TYPE_BRIDGED_NODE,
} from '../utils/deviceTree'
import type { EndpointSummary } from '../utils/deviceTree'

const props = defineProps<{ tree: EndpointTree }>()

function hex(n: number, pad = 4) {
  return `0x${n.toString(16).toUpperCase().padStart(pad, '0')}`
}

// Cache of device type id -> human-readable name
const dtNames = ref<Map<number, string>>(new Map())

async function loadDtNames() {
  const ids = new Set<number>()
  const { byEndpoint } = buildDeviceTree(props.tree)
  for (const summary of byEndpoint.values()) {
    for (const dt of summary.deviceTypes) {
      ids.add(dt.id)
    }
  }
  const results = await Promise.all(
    [...ids].map(id =>
      invoke<string | null>('get_device_type_name', { deviceType: id }).then(name => [id, name] as [number, string | null])
    )
  )
  const map = new Map<number, string>()
  for (const [id, name] of results) {
    if (name) map.set(id, name)
  }
  dtNames.value = map
}

onMounted(loadDtNames)
watch(() => props.tree, loadDtNames)

function dtName(id: number): string {
  return dtNames.value.get(id) ?? hex(id)
}

const treeData = computed<TreeOption[]>(() => {
  const { byEndpoint, childrenOf, roots } = buildDeviceTree(props.tree)

  function makeNode(epId: number): TreeOption {
    const s = byEndpoint.get(epId) as EndpointSummary
    const primary = primaryDeviceType(s)
    const children = (childrenOf.get(epId) ?? []).map(makeNode)
    return {
      key: epId,
      label: epId.toString(),
      _summary: s,
      _primaryId: primary?.id ?? null,
      children: children.length ? children : undefined,
    }
  }

  return roots.map(makeNode)
})

function renderLabel({ option }: { option: TreeOption }) {
  const s = (option as any)._summary as EndpointSummary | undefined
  if (!s) return option.label as string

  const primaryId: number | null = (option as any)._primaryId
  const isBridged = s.deviceTypes.some(d => d.id === DEVICE_TYPE_BRIDGED_NODE)

  const parts: any[] = [
    h('span', { style: 'font-family: monospace; font-size: 12px; margin-right: 6px' }, `ep ${s.endpoint}`),
  ]

  if (primaryId != null) {
    parts.push(
      h('span', { style: 'font-weight: 500; margin-right: 6px' }, dtName(primaryId))
    )
  }

  const name = s.label || s.productName
  if (name) {
    parts.push(h('span', { style: 'color: var(--n-text-color-3); margin-right: 6px' }, name))
  }

  if (s.vendorName) {
    parts.push(h('span', { style: 'color: var(--n-text-color-3); font-size: 12px; margin-right: 6px' }, `by ${s.vendorName}`))
  }

  if (isBridged) {
    const reachable = s.reachable
    const type = reachable === false ? 'error' : reachable === true ? 'success' : 'default'
    const text = reachable === false ? 'Unreachable' : reachable === true ? 'Reachable' : 'Unknown'
    parts.push(h(NTag, { size: 'small', type, bordered: false }, () => text))
  }

  return h(NSpace, { align: 'center', size: 4, style: 'flex-wrap: nowrap' }, () => parts)
}
</script>

<template>
  <n-tree
    :data="treeData"
    :default-expand-all="true"
    :render-label="renderLabel"
    block-line
    :selectable="false"
    style="height: calc(100vh - 220px); min-height: 200px; overflow: auto"
  />
</template>
