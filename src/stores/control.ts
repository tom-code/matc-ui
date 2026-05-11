import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useDevicesStore } from './devices'
import {
  CLUSTER_ID_IDENTIFY,
  CLUSTER_ID_ON_OFF,
  CLUSTER_ID_LEVEL_CONTROL,
  CLUSTER_ID_COLOR_CONTROL,
  CLUSTER_ID_BRIDGED_DEVICE_BASIC_INFO,
  ATTR_NODE_LABEL,
  ATTR_COLOR_TEMP_MIREDS,
  CONTROLLABLE_CLUSTER_IDS,
  parseAttrJson,
} from '../utils/controlClusters'
import type { EndpointTree } from '../types'

export interface ControllableEndpoint {
  nodeId: number
  deviceName: string
  endpointId: number
  clusters: number[]
  nodeLabel?: string
}

export interface EndpointControlState {
  onOff?: boolean
  level?: number | null
  minLevel?: number
  maxLevel?: number
  hue?: number
  saturation?: number
  colorTempMireds?: number
  loading: boolean
  error?: string
}

type StateKey = string

function stateKey(nodeId: number, endpointId: number): StateKey {
  return `${nodeId}:${endpointId}`
}

export const useControlStore = defineStore('control', () => {
  const controllables = ref<ControllableEndpoint[]>([])
  const state = ref<Record<StateKey, EndpointControlState>>({})
  const loading = ref(false)

  async function loadControllables() {
    loading.value = true
    try {
      const devicesStore = useDevicesStore()
      await devicesStore.fetchDevices()

      const result: ControllableEndpoint[] = []

      await Promise.all(
        devicesStore.devices.map(async device => {
          try {
            const tree = await invoke<EndpointTree>('read_endpoint_structure', {
              nodeId: device.node_id,
            })
            for (const ep of tree.endpoints) {
              const allClusterIds = ep.clusters.map(c => c.id)
              const presentClusters = allClusterIds.filter(id => CONTROLLABLE_CLUSTER_IDS.includes(id))
              if (presentClusters.length === 0) continue
              if (allClusterIds.includes(CLUSTER_ID_IDENTIFY)) presentClusters.push(CLUSTER_ID_IDENTIFY)

              let nodeLabel: string | undefined
              if (allClusterIds.includes(CLUSTER_ID_BRIDGED_DEVICE_BASIC_INFO)) {
                try {
                  const raw = await invoke<string>('read_single_attribute', {
                    nodeId: device.node_id,
                    endpoint: ep.id,
                    cluster: CLUSTER_ID_BRIDGED_DEVICE_BASIC_INFO,
                    attrId: ATTR_NODE_LABEL,
                  })
                  const label = parseAttrJson<string>(raw)
                  if (label && label.length > 0) nodeLabel = label
                } catch {
                  // fall back to endpoint ID label
                }
              }

              result.push({
                nodeId: device.node_id,
                deviceName: device.name,
                endpointId: ep.id,
                clusters: presentClusters,
                nodeLabel,
              })
            }
          } catch {
            // skip devices that cannot be enumerated
          }
        })
      )

      result.sort((a, b) =>
        a.nodeId !== b.nodeId ? a.nodeId - b.nodeId : a.endpointId - b.endpointId
      )
      controllables.value = result

      await Promise.all(
        result.map(ep => refreshState(ep.nodeId, ep.endpointId, ep.clusters))
      )
    } finally {
      loading.value = false
    }
  }

  async function refreshState(nodeId: number, endpointId: number, clusters: number[]) {
    const key = stateKey(nodeId, endpointId)
    const prev = state.value[key] ?? { loading: false }
    state.value[key] = { ...prev, loading: true, error: undefined }

    const updates: Partial<EndpointControlState> = {}
    const errors: string[] = []

    async function readAttr(cluster: number, attrId: number): Promise<string> {
      return invoke<string>('read_single_attribute', { nodeId, endpoint: endpointId, cluster, attrId })
    }

    if (clusters.includes(CLUSTER_ID_ON_OFF)) {
      try {
        const raw = await readAttr(CLUSTER_ID_ON_OFF, 0x0000)
        const parsed = parseAttrJson<boolean>(raw)
        if (parsed != null) updates.onOff = parsed
      } catch (e) {
        errors.push(String(e))
      }
    }

    if (clusters.includes(CLUSTER_ID_LEVEL_CONTROL)) {
      try {
        const raw = await readAttr(CLUSTER_ID_LEVEL_CONTROL, 0x0000)
        updates.level = parseAttrJson<number | null>(raw) ?? null
      } catch (e) {
        errors.push(String(e))
      }
      try {
        const raw = await readAttr(CLUSTER_ID_LEVEL_CONTROL, 0x0002)
        updates.minLevel = parseAttrJson<number>(raw) ?? 1
      } catch {
        updates.minLevel = 1
      }
      try {
        const raw = await readAttr(CLUSTER_ID_LEVEL_CONTROL, 0x0003)
        updates.maxLevel = parseAttrJson<number>(raw) ?? 254
      } catch {
        updates.maxLevel = 254
      }
    }

    if (clusters.includes(CLUSTER_ID_COLOR_CONTROL)) {
      let hueOk = false
      let satOk = false

      try {
        const raw = await readAttr(CLUSTER_ID_COLOR_CONTROL, 0x0000)
        const hue = parseAttrJson<number>(raw)
        if (hue != null) { updates.hue = hue; hueOk = true }
      } catch { /* UNSUPPORTED_ATTRIBUTE on CT-only lights */ }

      try {
        const raw = await readAttr(CLUSTER_ID_COLOR_CONTROL, 0x0001)
        const sat = parseAttrJson<number>(raw)
        if (sat != null) { updates.saturation = sat; satOk = true }
      } catch { /* same */ }

      if (!hueOk || !satOk) {
        try {
          const raw = await readAttr(CLUSTER_ID_COLOR_CONTROL, ATTR_COLOR_TEMP_MIREDS)
          const ct = parseAttrJson<number>(raw)
          if (ct != null && ct > 0) updates.colorTempMireds = ct
        } catch { /* not available either */ }
      }
    }

    state.value[key] = {
      ...updates,
      loading: false,
      ...(errors.length > 0 ? { error: errors[0] } : {}),
    }
  }

  async function refreshEndpoint(nodeId: number, endpointId: number) {
    const ep = controllables.value.find(
      c => c.nodeId === nodeId && c.endpointId === endpointId
    )
    if (ep) await refreshState(nodeId, endpointId, ep.clusters)
  }

  async function refreshAll() {
    await Promise.all(
      controllables.value.map(ep => refreshState(ep.nodeId, ep.endpointId, ep.clusters))
    )
  }

  async function toggleOnOff(nodeId: number, endpointId: number) {
    const key = stateKey(nodeId, endpointId)
    state.value[key] = { ...(state.value[key] ?? { loading: false }), loading: true, error: undefined }
    try {
      await invoke('invoke_command_typed', {
        nodeId,
        endpoint: endpointId,
        cluster: CLUSTER_ID_ON_OFF,
        command: 0x02,
        args: {},
      })
      const raw = await invoke<string>('read_single_attribute', {
        nodeId, endpoint: endpointId, cluster: CLUSTER_ID_ON_OFF, attrId: 0x0000,
      })
      const parsed = parseAttrJson<boolean>(raw)
      state.value[key] = {
        ...(state.value[key] ?? { loading: false }),
        ...(parsed != null ? { onOff: parsed } : {}),
        loading: false,
      }
    } catch (e) {
      state.value[key] = {
        ...(state.value[key] ?? { loading: false }),
        loading: false,
        error: String(e),
      }
    }
  }

  async function setLevel(nodeId: number, endpointId: number, level: number) {
    const key = stateKey(nodeId, endpointId)
    state.value[key] = { ...(state.value[key] ?? { loading: false }), loading: true, error: undefined }
    try {
      await invoke('invoke_command_typed', {
        nodeId,
        endpoint: endpointId,
        cluster: CLUSTER_ID_LEVEL_CONTROL,
        command: 0x00,
        args: { level, transition_time: null, options_mask: 0, options_override: 0 },
      })
      const raw = await invoke<string>('read_single_attribute', {
        nodeId, endpoint: endpointId, cluster: CLUSTER_ID_LEVEL_CONTROL, attrId: 0x0000,
      })
      state.value[key] = {
        ...(state.value[key] ?? { loading: false }),
        level: parseAttrJson<number | null>(raw) ?? null,
        loading: false,
      }
    } catch (e) {
      state.value[key] = {
        ...(state.value[key] ?? { loading: false }),
        loading: false,
        error: String(e),
      }
    }
  }

  async function identify(nodeId: number, endpointId: number) {
    const key = stateKey(nodeId, endpointId)
    state.value[key] = { ...(state.value[key] ?? { loading: false }), loading: true, error: undefined }
    try {
      await invoke('invoke_command_typed', {
        nodeId,
        endpoint: endpointId,
        cluster: CLUSTER_ID_IDENTIFY,
        command: 0x00,
        args: { identify_time: 5 },
      })
      state.value[key] = { ...(state.value[key] ?? { loading: false }), loading: false }
    } catch (e) {
      state.value[key] = {
        ...(state.value[key] ?? { loading: false }),
        loading: false,
        error: String(e),
      }
    }
  }

  function getState(nodeId: number, endpointId: number): EndpointControlState {
    return state.value[stateKey(nodeId, endpointId)] ?? { loading: false }
  }

  return {
    controllables,
    state,
    loading,
    loadControllables,
    refreshEndpoint,
    refreshAll,
    toggleOnOff,
    setLevel,
    identify,
    getState,
  }
})
