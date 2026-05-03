<script setup lang="ts">
import { h, computed } from 'vue'
import { NDataTable } from 'naive-ui'
import type { TreeOption, DataTableColumns } from 'naive-ui'
import type { EndpointTree, AttrNode } from '../types'

const props = defineProps<{ tree: EndpointTree }>()

function hex(n: number, pad = 4) {
  return `0x${n.toString(16).toUpperCase().padStart(pad, '0')}`
}

const MATTER_STATUS: Record<number, string> = {
  0x00: 'SUCCESS',
  0x01: 'FAILURE',
  0x7d: 'INVALID_SUBSCRIPTION',
  0x7e: 'UNSUPPORTED_ACCESS',
  0x7f: 'UNSUPPORTED_ENDPOINT',
  0x80: 'INVALID_ACTION',
  0x81: 'UNSUPPORTED_COMMAND',
  0x85: 'INVALID_COMMAND',
  0x86: 'UNSUPPORTED_ATTRIBUTE',
  0x87: 'CONSTRAINT_ERROR',
  0x88: 'UNSUPPORTED_WRITE',
  0x89: 'RESOURCE_EXHAUSTED',
  0x8c: 'UNREPORTABLE_ATTRIBUTE',
  0x8d: 'INVALID_DATA_TYPE',
  0x8f: 'UNSUPPORTED_READ',
  0x92: 'DATA_VERSION_MISMATCH',
  0x94: 'TIMEOUT',
  0x9c: 'BUSY',
  0x9d: 'ACCESS_RESTRICTED',
  0xc3: 'UNSUPPORTED_CLUSTER',
  0xc5: 'NO_UPSTREAM_SUBSCRIPTION',
}

function formatError(raw: string): string {
  const m = raw.match(/^report data with status (\d+)$/)
  if (m) {
    const code = parseInt(m[1], 10)
    const name = MATTER_STATUS[code]
    if (name) return `${name} (${hex(code, 2)})`
    if (code >= 0x80 && code <= 0xbf) return `CLUSTER_SPECIFIC (${hex(code, 2)})`
    return `STATUS_${hex(code, 2)}`
  }
  return raw
}

const columns: DataTableColumns<AttrNode> = [
  {
    title: 'ID',
    key: 'id',
    width: 90,
    render: row => h('span', { style: 'font-family: monospace; font-size: 12px' }, hex(row.id)),
  },
  {
    title: 'Name',
    key: 'name',
    width: 220,
    ellipsis: { tooltip: true },
  },
  {
    title: 'Value',
    key: 'value',
    ellipsis: { tooltip: true },
    render: row => {
      if (row.error != null) {
        return h('span', { style: 'font-family: monospace; font-size: 12px; color: #e03e3e' }, `<${formatError(row.error)}>`)
      }
      return h('span', { style: 'font-family: monospace; font-size: 12px' }, row.value ?? '')
    },
  },
]

const treeData = computed<TreeOption[]>(() =>
  props.tree.endpoints.map(ep => ({
    key: `ep-${ep.id}`,
    label: `Endpoint ${ep.id}`,
    children: ep.clusters.map(cl => ({
      key: `ep-${ep.id}-cl-${cl.id}`,
      label: `${cl.name} (${hex(cl.id)})`,
      children: cl.attributes.length
        ? [{
            key: `ep-${ep.id}-cl-${cl.id}-table`,
            label: '',
            isLeaf: true,
            disabled: true,
            _attrs: cl.attributes,
          }]
        : [{
            key: `ep-${ep.id}-cl-${cl.id}-empty`,
            label: '',
            isLeaf: true,
            disabled: true,
            _empty: true,
          }],
    })),
  }))
)

function renderLabel({ option }: { option: TreeOption }) {
  if ((option as any)._attrs) {
    return h(NDataTable, {
      columns,
      data: (option as any)._attrs,
      rowKey: (r: AttrNode) => r.id,
      size: 'small',
      bordered: false,
      style: 'margin: 4px 0',
    })
  }
  if ((option as any)._empty) {
    return h('span', { style: 'opacity: 0.6; font-style: italic' }, '(no known attributes)')
  }
  return option.label as string
}
</script>

<template>
  <n-tree
    :data="treeData"
    :default-expand-all="false"
    :render-label="renderLabel"
    block-line
    :selectable="false"
    style="max-height: 65vh; overflow: auto"
  />
</template>
