<script setup lang="ts">
import { computed } from 'vue'
import type { TreeOption } from 'naive-ui'
import type { EndpointTree } from '../types'

const props = defineProps<{ tree: EndpointTree }>()

function hex(n: number, pad = 4) {
  return `0x${n.toString(16).toUpperCase().padStart(pad, '0')}`
}

const treeData = computed<TreeOption[]>(() =>
  props.tree.endpoints.map(ep => ({
    key: `ep-${ep.id}`,
    label: `Endpoint ${ep.id}`,
    children: ep.clusters.map(cl => ({
      key: `ep-${ep.id}-cl-${cl.id}`,
      label: `${cl.name} (${hex(cl.id)})`,
      children: cl.attributes.length
        ? cl.attributes.map(attr => ({
            key: `ep-${ep.id}-cl-${cl.id}-attr-${attr.id}`,
            label: `${attr.name} (${hex(attr.id)}) = ${attr.value}`,
            isLeaf: true,
          }))
        : [{
            key: `ep-${ep.id}-cl-${cl.id}-empty`,
            label: '(no known attributes)',
            isLeaf: true,
            disabled: true,
          }],
    })),
  }))
)
</script>

<template>
  <n-tree
    :data="treeData"
    :default-expand-all="false"
    block-line
    selectable
    virtual-scroll
    style="max-height: 65vh; overflow: auto"
  />
</template>
