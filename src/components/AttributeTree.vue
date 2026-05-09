<script setup lang="ts">
import { h, computed } from 'vue'
import { NDataTable } from 'naive-ui'
import type { TreeOption, DataTableColumns } from 'naive-ui'
import type { EndpointTree, AttrNode } from '../types'
import { formatStatus } from '../utils/matterStatus'

const props = defineProps<{ tree: EndpointTree }>()

function hex(n: number, pad = 4) {
  return `0x${n.toString(16).toUpperCase().padStart(pad, '0')}`
}

function formatError(raw: string): string {
  const m = raw.match(/^report data with status (\d+)$/)
  if (m) {
    return formatStatus(parseInt(m[1], 10))
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
    style="height: calc(100vh - 220px); min-height: 200px; overflow: auto"
  />
</template>
