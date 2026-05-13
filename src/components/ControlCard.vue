<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useThemeVars } from 'naive-ui'
import type { EndpointControlState } from '../stores/control'
import {
  CLUSTER_ID_IDENTIFY,
  CLUSTER_ID_ON_OFF,
  CLUSTER_ID_LEVEL_CONTROL,
  CLUSTER_ID_COLOR_CONTROL,
  CLUSTER_ID_SWITCH,
  CLUSTER_ID_ILLUMINANCE_MEASUREMENT,
  CLUSTER_ID_TEMPERATURE_MEASUREMENT,
  CLUSTER_ID_OCCUPANCY_SENSING,
  kelvinToRgb,
  luxFromRaw,
  tempCFromRaw,
  matterHueSatToHex,
  rgbHexToMatterHueSat,
} from '../utils/controlClusters'

const props = defineProps<{
  title: string
  nodeId: number
  endpointId: number
  clusters: number[]
  state: EndpointControlState
  deviceTypes: Array<{ id: number; name?: string }>
  productLabel?: string
  offline?: boolean
}>()

const emit = defineEmits<{
  (e: 'toggle'): void
  (e: 'set-level', level: number): void
  (e: 'set-color-hs', hue: number, saturation: number): void
  (e: 'set-color-temp', mireds: number): void
  (e: 'refresh'): void
  (e: 'identify'): void
}>()

const hasIdentify = computed(() => props.clusters.includes(CLUSTER_ID_IDENTIFY))
const hasOnOff = computed(() => props.clusters.includes(CLUSTER_ID_ON_OFF))
const hasLevel = computed(() => props.clusters.includes(CLUSTER_ID_LEVEL_CONTROL))
const hasColor = computed(() => props.clusters.includes(CLUSTER_ID_COLOR_CONTROL))
const hasSwitch = computed(() => props.clusters.includes(CLUSTER_ID_SWITCH))
const hasOccupancy = computed(() => props.clusters.includes(CLUSTER_ID_OCCUPANCY_SENSING))
const hasIlluminance = computed(() => props.clusters.includes(CLUSTER_ID_ILLUMINANCE_MEASUREMENT))
const hasTemperature = computed(() => props.clusters.includes(CLUSTER_ID_TEMPERATURE_MEASUREMENT))

const switchPositionLabel = computed(() => {
  const cur = props.state.currentPosition
  const total = props.state.numberOfPositions
  if (cur == null || total == null) return '-'
  return `${cur} / ${total - 1}`
})

const occupancyLabel = computed(() => {
  if (props.state.occupancy == null) return '-'
  return props.state.occupancy ? 'Occupied' : 'Unoccupied'
})

const illuminanceText = computed(() => {
  const lux = luxFromRaw(props.state.illuminanceMeasured ?? null)
  if (lux == null) return '-'
  if (lux === 0) return 'too low'
  return `${Math.round(lux)} lux`
})

const illuminanceRange = computed(() => {
  const min = luxFromRaw(props.state.illuminanceMin ?? null)
  const max = luxFromRaw(props.state.illuminanceMax ?? null)
  if (min == null || max == null) return null
  return `${Math.round(min)}..${Math.round(max)} lux`
})

const temperatureText = computed(() => {
  const c = tempCFromRaw(props.state.temperatureMeasured ?? null)
  if (c == null) return '-'
  return `${c.toFixed(1)} C`
})

const temperatureRange = computed(() => {
  const min = tempCFromRaw(props.state.temperatureMin ?? null)
  const max = tempCFromRaw(props.state.temperatureMax ?? null)
  if (min == null || max == null) return null
  return `${min.toFixed(1)}..${max.toFixed(1)} C`
})

const themeVars = useThemeVars()

const showInfo = ref(false)

function hex4(n: number): string {
  return '0x' + n.toString(16).toUpperCase().padStart(4, '0')
}

const deviceTypeLabels = computed(() => props.deviceTypes.map(d => d.name ?? hex4(d.id)))

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
  const showCt = props.state.colorMode === 2 ||
    (props.state.colorMode == null && props.state.hue == null && props.state.colorTempMireds != null)
  if (!showCt && props.state.hue != null && props.state.saturation != null) {
    const h = Math.round((props.state.hue / 254) * 360)
    const s = Math.round((props.state.saturation / 254) * 100)
    return { css: `hsl(${h}, ${s}%, 50%)`, label: 'HS' }
  }
  if (props.state.colorTempMireds != null && props.state.colorTempMireds > 0) {
    const kelvin = Math.round(1000000 / props.state.colorTempMireds)
    const [r, g, b] = kelvinToRgb(kelvin)
    return { css: `rgb(${r}, ${g}, ${b})`, label: `${kelvin}K` }
  }
  if (props.state.hue != null && props.state.saturation != null) {
    const h = Math.round((props.state.hue / 254) * 360)
    const s = Math.round((props.state.saturation / 254) * 100)
    return { css: `hsl(${h}, ${s}%, 50%)`, label: 'HS' }
  }
  return null
})

const showHsPicker = computed(() => props.state.hue != null && props.state.saturation != null)
const showCtPicker = computed(() => props.state.colorTempMireds != null && props.state.colorTempMireds > 0)

const colorPickerOpen = ref(false)
const pickerMode = ref<'hs' | 'ct'>('hs')
const hsDraft = ref('#ffffff')
const ctDraft = ref(300)

const ctPickerMin = computed(() => props.state.colorTempMinMireds ?? 153)
const ctPickerMax = computed(() => props.state.colorTempMaxMireds ?? 500)
const ctDraftKelvin = computed(() => Math.round(1000000 / ctDraft.value))

watch(colorPickerOpen, (open) => {
  if (!open) return
  pickerMode.value = (props.state.colorMode === 2 || (!showHsPicker.value && showCtPicker.value)) ? 'ct' : 'hs'
  if (props.state.hue != null && props.state.saturation != null) {
    hsDraft.value = matterHueSatToHex(props.state.hue, props.state.saturation)
  }
  if (props.state.colorTempMireds != null && props.state.colorTempMireds > 0) {
    ctDraft.value = props.state.colorTempMireds
  }
})

function applyHueSat() {
  const [h, s] = rgbHexToMatterHueSat(hsDraft.value)
  emit('set-color-hs', h, s)
  colorPickerOpen.value = false
}

function applyColorTemp() {
  emit('set-color-temp', ctDraft.value)
  colorPickerOpen.value = false
}

const onOffLabel = computed(() => {
  if (props.state.onOff === true) return 'On'
  if (props.state.onOff === false) return 'Off'
  return '-'
})
</script>

<template>
  <n-card :title="title" hoverable :class="{ 'card-offline': offline }">
    <template #header-extra>
      <n-space size="small" align="center">
        <n-tag v-if="offline" size="small" type="warning" :bordered="false">Offline</n-tag>
        <template v-if="!showInfo">
          <n-button
            v-if="hasIdentify"
            size="tiny"
            :loading="state.loading"
            :disabled="state.loading || offline"
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
        </template>
        <button
          class="icon-btn"
          :title="showInfo ? 'Back to controls' : 'Show details'"
          @click="showInfo = !showInfo"
        >
          <svg v-if="!showInfo" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="9"/>
            <line x1="12" y1="11" x2="12" y2="16"/>
            <line x1="12" y1="8" x2="12" y2="8" stroke-width="3"/>
          </svg>
          <svg v-else width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="6" y1="6" x2="18" y2="18"/>
            <line x1="18" y1="6" x2="6" y2="18"/>
          </svg>
        </button>
      </n-space>
    </template>

    <div v-if="showInfo" class="control-card-body info-list">
      <div class="info-row">
        <span class="info-label">Node ID</span>
        <router-link :to="`/devices/${nodeId}`" class="info-value node-id-link" :style="{ color: themeVars.primaryColor }">{{ nodeId }}</router-link>
      </div>
      <div class="info-row">
        <span class="info-label">Endpoint ID</span>
        <span class="info-value">{{ endpointId }}</span>
      </div>
      <div class="info-row">
        <span class="info-label">Device types</span>
        <span class="info-value">{{ deviceTypeLabels.length ? deviceTypeLabels.join(', ') : '-' }}</span>
      </div>
      <div v-if="productLabel && productLabel.length" class="info-row">
        <span class="info-label">Product label</span>
        <span class="info-value">{{ productLabel }}</span>
      </div>
    </div>

    <div v-else class="control-card-body">
      <div v-if="hasOnOff || hasColor" class="cluster-row">
        <button
          v-if="hasOnOff"
          class="power-btn"
          :class="{ 'is-on': state.onOff === true }"
          :disabled="state.loading || offline"
          :title="state.onOff ? 'On - click to toggle' : 'Off - click to toggle'"
          @click="emit('toggle')"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M18.36 6.64a9 9 0 1 1-12.73 0"/>
            <line x1="12" y1="2" x2="12" y2="12"/>
          </svg>
        </button>
        <span v-if="hasOnOff" class="state-label">{{ onOffLabel }}</span>
        <n-popover
          v-if="hasColor"
          v-model:show="colorPickerOpen"
          trigger="click"
          placement="bottom-start"
          :disabled="offline"
        >
          <template #trigger>
            <div
              class="color-swatch color-swatch--btn"
              :class="{ 'color-swatch--disabled': offline }"
              :style="{ background: colorSwatch ? colorSwatch.css : '#cccccc' }"
              :title="colorSwatch ? colorSwatch.label : 'Color unavailable'"
            />
          </template>
          <div class="color-pop">
            <div v-if="showHsPicker && showCtPicker" class="color-mode-tabs">
              <button :class="['mode-tab', { 'mode-tab--active': pickerMode === 'hs' }]" @click.stop="pickerMode = 'hs'">HS</button>
              <button :class="['mode-tab', { 'mode-tab--active': pickerMode === 'ct' }]" @click.stop="pickerMode = 'ct'">CT</button>
            </div>
            <div v-if="pickerMode === 'hs' && showHsPicker" class="picker-section">
              <input
                type="color"
                :value="hsDraft"
                class="color-input"
                @input="e => hsDraft = (e.target as HTMLInputElement).value"
              />
              <n-button size="small" type="primary" :disabled="state.loading || offline" @click="applyHueSat">Apply</n-button>
            </div>
            <div v-else-if="pickerMode === 'ct' && showCtPicker" class="picker-section ct-section">
              <n-slider
                v-model:value="ctDraft"
                :min="ctPickerMin"
                :max="ctPickerMax"
                :step="1"
                style="width: 160px"
              />
              <span class="ct-kelvin">{{ ctDraftKelvin }}K</span>
              <n-button size="small" type="primary" :disabled="state.loading || offline" @click="applyColorTemp">Apply</n-button>
            </div>
            <span v-else class="state-label" style="font-size:12px">No color data</span>
          </div>
        </n-popover>
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
            :disabled="state.loading || offline"
            @update:value="onLevelInput"
            @dragend="onLevelDragEnd"
          />
        </div>
      </div>

      <div v-if="hasSwitch" class="cluster-row switch-row">
        <span class="state-label">Position:</span>
        <span class="switch-position">{{ switchPositionLabel }}</span>
        <span v-if="state.multiPressMax != null && state.multiPressMax > 1" class="state-label" style="margin-left: 12px">
          Multi-press: {{ state.multiPressMax }}
        </span>
      </div>

      <div v-if="hasOccupancy" class="cluster-row">
        <span
          class="occupancy-dot"
          :class="state.occupancy ? 'occupancy-dot--on' : 'occupancy-dot--off'"
        />
        <span class="sensor-value">{{ occupancyLabel }}</span>
        <span v-if="state.occupancySensorType" class="state-label" style="margin-left: 8px">
          ({{ state.occupancySensorType }})
        </span>
      </div>

      <div v-if="hasIlluminance" class="cluster-row">
        <span class="state-label">Illuminance:</span>
        <span class="sensor-value">{{ illuminanceText }}</span>
        <span v-if="illuminanceRange" class="state-label" style="margin-left: 8px">
          {{ illuminanceRange }}
        </span>
      </div>

      <div v-if="hasTemperature" class="cluster-row">
        <span class="state-label">Temperature:</span>
        <span class="sensor-value">{{ temperatureText }}</span>
        <span v-if="temperatureRange" class="state-label" style="margin-left: 8px">
          {{ temperatureRange }}
        </span>
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
:deep(.card-offline) {
  opacity: 0.6;
  filter: grayscale(0.25);
}

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

.color-swatch--btn {
  cursor: pointer;
  transition: transform 0.1s, box-shadow 0.1s;
}

.color-swatch--btn:hover {
  transform: scale(1.15);
  box-shadow: 0 0 0 3px rgba(0, 0, 0, 0.12);
}

.color-swatch--disabled {
  cursor: not-allowed;
  opacity: 0.5;
}

.color-swatch--disabled:hover {
  transform: none;
  box-shadow: none;
}


.switch-row {
  font-size: 13px;
}

.switch-position {
  font-weight: 600;
  font-size: 13px;
}

.occupancy-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}

.occupancy-dot--on {
  background: #18a058;
}

.occupancy-dot--off {
  background: #ccc;
}

.sensor-value {
  font-weight: 600;
  font-size: 13px;
}

.info-list {
  gap: 5px;
  min-height: unset;
}

.info-row {
  display: grid;
  grid-template-columns: 110px 1fr;
  align-items: baseline;
  gap: 8px;
}

.info-label {
  font-size: 13px;
  color: var(--n-text-color-3, #888);
}

.info-value {
  font-size: 14px;
  word-break: break-word;
}

.icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border: none;
  background: transparent;
  cursor: pointer;
  color: var(--n-text-color-3, #888);
  border-radius: 4px;
  padding: 0;
}

.icon-btn:hover {
  color: var(--n-text-color-2, #555);
  background: rgba(0, 0, 0, 0.04);
}

.node-id-link {
  text-decoration: none;
}

.node-id-link:hover {
  text-decoration: underline;
}

.color-pop {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 4px 2px;
  min-width: 200px;
}

.color-mode-tabs {
  display: flex;
  gap: 4px;
}

.mode-tab {
  flex: 1;
  padding: 2px 10px;
  border: 1px solid #ccc;
  border-radius: 4px;
  background: transparent;
  cursor: pointer;
  font-size: 12px;
  color: inherit;
}

.mode-tab--active {
  border-color: var(--n-primary-color, #18a058);
  color: var(--n-primary-color, #18a058);
  font-weight: 600;
}

.picker-section {
  display: flex;
  align-items: center;
  gap: 10px;
}

.ct-section {
  flex-direction: column;
  align-items: flex-start;
}

.color-input {
  width: 42px;
  height: 42px;
  border: none;
  border-radius: 4px;
  padding: 0;
  cursor: pointer;
  background: none;
}

.ct-kelvin {
  font-size: 13px;
  font-weight: 600;
}
</style>
