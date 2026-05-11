<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useControlStore } from '../stores/control'
import ControlCard from '../components/ControlCard.vue'

const store = useControlStore()

onMounted(() => {
  store.loadControllables()
})

const endpointsPerDevice = computed(() => {
  const counts = new Map<number, number>()
  for (const ep of store.controllables) {
    counts.set(ep.nodeId, (counts.get(ep.nodeId) ?? 0) + 1)
  }
  return counts
})

function cardTitle(ep: { nodeId: number; deviceName: string; endpointId: number; nodeLabel?: string }): string {
  const count = endpointsPerDevice.value.get(ep.nodeId) ?? 1
  if (count === 1) return ep.deviceName
  if (ep.nodeLabel) return `${ep.deviceName} - ${ep.nodeLabel}`
  return `${ep.deviceName} - Endpoint ${ep.endpointId}`
}
</script>

<template>
  <div class="view-container">
    <div class="view-header">
      <n-h2 style="margin: 0">Control</n-h2>
      <n-button
        :loading="store.loading"
        size="small"
        @click="store.refreshAll()"
      >
        Refresh
      </n-button>
    </div>

    <n-spin :show="store.loading">
      <div v-if="!store.loading && store.controllables.length === 0" class="empty-state">
        <n-empty description="No controllable endpoints found">
          <template #extra>
            <n-text depth="3" style="font-size: 12px">
              Commission devices with OnOff, LevelControl, or ColorControl clusters.
            </n-text>
          </template>
        </n-empty>
      </div>

      <div class="control-grid">
        <ControlCard
          v-for="ep in store.controllables"
          :key="`${ep.nodeId}:${ep.endpointId}`"
          :title="cardTitle(ep)"
          :node-id="ep.nodeId"
          :endpoint-id="ep.endpointId"
          :clusters="ep.clusters"
          :state="store.getState(ep.nodeId, ep.endpointId)"
          @toggle="store.toggleOnOff(ep.nodeId, ep.endpointId)"
          @set-level="level => store.setLevel(ep.nodeId, ep.endpointId, level)"
          @identify="store.identify(ep.nodeId, ep.endpointId)"
          @refresh="store.refreshEndpoint(ep.nodeId, ep.endpointId)"
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

.control-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 16px;
}

.empty-state {
  padding: 60px 0;
}
</style>
