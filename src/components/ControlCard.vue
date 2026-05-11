<script setup lang="ts">
import { ref, computed } from 'vue'
import type { EndpointControlState } from '../stores/control'
import { CLUSTER_ID_IDENTIFY, CLUSTER_ID_ON_OFF, CLUSTER_ID_LEVEL_CONTROL, CLUSTER_ID_COLOR_CONTROL, kelvinToRgb } from '../utils/controlClusters'

const props = defineProps<{
  title: string
  nodeId: number
  endpointId: number
  clusters: number[]
  state: EndpointControlState
}>()

const emit = defineEmits<{
  (e: 'toggle'): void
  (e: 'set-level', level: number): void
  (e: 'refresh'): void
  (e: 'identify'): void
}>()

const hasIdentify = computed(() => props.clusters.includes(CLUSTER_ID_IDENTIFY))
const hasOnOff = computed(() => props.clusters.includes(CLUSTER_ID_ON_OFF))
const hasLevel = computed(() => props.clusters.includes(CLUSTER_ID_LEVEL_CONTROL))
const hasColor = computed(() => props.clusters.includes(CLUSTER_ID_COLOR_CONTROL))

const levelDraft = ref<number | null>(null)

const displayLevel = computed(() =>
  levelDraft.value ?? (typeof props.state.level === 'number' ? props.state.level : 0)
)

const levelPct = computed(() => {
  const max = props.state.maxLevel ?? 254
  return Math.round((displayLevel.value / max) * 100)
})

function onLevelInput(v: number | number[]) {
  if (typeof v === 'number') levelDraft.value = v
}

function onLevelDragEnd() {
  if (levelDraft.value != null) {
    emit('set-level', levelDraft.value)
    levelDraft.value = null
  }
}

const colorSwatch = computed<{ css: string; label: string } | null>(() => {
  if (props.state.hue != null && props.state.saturation != null) {
    const h = Math.round((props.state.hue / 254) * 360)
    const s = Math.round((props.state.saturation / 254) * 100)
    return { css: `hsl(${h}, ${s}%, 50%)`, label: 'HS color' }
  }
  if (props.state.colorTempMireds != null) {
    const kelvin = Math.round(1000000 / props.state.colorTempMireds)
    const [r, g, b] = kelvinToRgb(kelvin)
    return { css: `rgb(${r}, ${g}, ${b})`, label: `${kelvin}K` }
  }
  return null
})

const onOffLabel = computed(() => {
  if (props.state.onOff === true) return 'On'
  if (props.state.onOff === false) return 'Off'
  return '-'
})
</script>

<template>
  <n-card :title="title" hoverable>
    <template #header-extra>
      <n-space size="small" align="center">
        <n-tag size="small" :bordered="false" type="default">Node {{ nodeId }}</n-tag>
        <n-tag size="small" :bordered="false" type="default">EP {{ endpointId }}</n-tag>
        <n-button
          v-if="hasIdentify"
          size="tiny"
          :loading="state.loading"
          :disabled="state.loading"
          @click="emit('identify')"
        >
          Identify
        </n-button>
        <n-button
          size="tiny"
          :loading="state.loading"
          :disabled="state.loading"
          @click="emit('refresh')"
        >
          Refresh
        </n-button>
      </n-space>
    </template>

    <div class="control-card-body">
      <div v-if="hasOnOff || hasColor" class="cluster-row">
        <button
          v-if="hasOnOff"
          class="power-btn"
          :class="{ 'is-on': state.onOff === true }"
          :disabled="state.loading"
          :title="state.onOff ? 'On - click to toggle' : 'Off - click to toggle'"
          @click="emit('toggle')"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M18.36 6.64a9 9 0 1 1-12.73 0"/>
            <line x1="12" y1="2" x2="12" y2="12"/>
          </svg>
        </button>
        <span v-if="hasOnOff" class="state-label">{{ onOffLabel }}</span>
        <div
          v-if="hasColor"
          class="color-swatch"
          :style="{ background: colorSwatch ? colorSwatch.css : '#cccccc' }"
          :title="colorSwatch ? colorSwatch.label : 'Color unavailable'"
        />
        <span v-if="hasColor" class="state-label">{{ colorSwatch ? colorSwatch.label : 'No color data' }}</span>
      </div>

      <div v-if="hasLevel" class="cluster-row level-row">
        <n-progress
          type="circle"
          :percentage="levelPct"
          :stroke-width="10"
          style="width: 24px; flex-shrink: 0"
          :show-indicator="false"
        />
        <div class="level-slider-wrap">
          <n-slider
            :value="displayLevel"
            :min="state.minLevel ?? 1"
            :max="state.maxLevel ?? 254"
            :step="1"
            :disabled="state.loading"
            @update:value="onLevelInput"
            @dragend="onLevelDragEnd"
          />
        </div>
      </div>

      <n-text
        v-if="state.error"
        type="error"
        style="font-size: 12px; display: block; margin-top: 4px; word-break: break-all"
      >
        {{ state.error }}
      </n-text>
    </div>
  </n-card>
</template>

<style scoped>
.control-card-body {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 60px;
}

.cluster-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.level-row {
  align-items: center;
}

.level-slider-wrap {
  flex: 1;
}

.power-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border-radius: 50%;
  border: 2px solid #ccc;
  background: transparent;
  cursor: pointer;
  color: #888;
  transition: border-color 0.15s, color 0.15s, background 0.15s;
  flex-shrink: 0;
}

.power-btn:hover:not(:disabled) {
  border-color: #aaa;
  color: #555;
}

.power-btn.is-on {
  border-color: #f0a020;
  color: #f0a020;
  background: rgba(240, 160, 32, 0.08);
}

.power-btn.is-on:hover:not(:disabled) {
  background: rgba(240, 160, 32, 0.16);
}

.power-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.state-label {
  font-size: 13px;
  color: var(--n-text-color-3, #888);
}

.level-text {
  font-size: 12px;
  font-weight: 600;
}

.color-swatch {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  border: 2px solid rgba(0, 0, 0, 0.1);
  flex-shrink: 0;
}
</style>
