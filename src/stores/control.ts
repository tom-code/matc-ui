import { defineStore } from 'pinia'
import { ref, watch } from 'vue'

// Run tasks with at most `limit` concurrent executions.
async function runConcurrent<T>(tasks: Array<() => Promise<T>>, limit: number): Promise<T[]> {
  const results: T[] = new Array(tasks.length)
  let next = 0
  async function worker() {
    while (next < tasks.length) {
      const i = next++
      results[i] = await tasks[i]()
    }
  }
  await Promise.all(Array.from({ length: Math.min(limit, tasks.length) }, worker))
  return results
}
import { invoke } from '@tauri-apps/api/core'
import { useDevicesStore } from './devices'
import type { DeviceDto } from '../types'
import {
  CLUSTER_ID_IDENTIFY,
  CLUSTER_ID_ON_OFF,
  CLUSTER_ID_LEVEL_CONTROL,
  CLUSTER_ID_COLOR_CONTROL,
  CLUSTER_ID_SWITCH,
  CLUSTER_ID_ILLUMINANCE_MEASUREMENT,
  CLUSTER_ID_TEMPERATURE_MEASUREMENT,
  CLUSTER_ID_OCCUPANCY_SENSING,
  CLUSTER_ID_BRIDGED_DEVICE_BASIC_INFO,
  ATTR_NODE_LABEL,
  ATTR_COLOR_MODE,
  ATTR_COLOR_TEMP_MIREDS,
  ATTR_COLOR_TEMP_MIN_MIREDS,
  ATTR_COLOR_TEMP_MAX_MIREDS,
  ATTR_SWITCH_NUMBER_OF_POSITIONS,
  ATTR_SWITCH_CURRENT_POSITION,
  ATTR_SWITCH_MULTI_PRESS_MAX,
  ATTR_OCCUPANCY,
  ATTR_OCCUPANCY_SENSOR_TYPE,
  ATTR_ILLUMINANCE_MEASURED_VALUE,
  ATTR_ILLUMINANCE_MIN_MEASURED_VALUE,
  ATTR_ILLUMINANCE_MAX_MEASURED_VALUE,
  ATTR_TEMPERATURE_MEASURED_VALUE,
  ATTR_TEMPERATURE_MIN_MEASURED_VALUE,
  ATTR_TEMPERATURE_MAX_MEASURED_VALUE,
  CONTROLLABLE_CLUSTER_IDS,
  parseAttrJson,
} from '../utils/controlClusters'
import {
  buildDeviceTree,
  findInheritedProductLabel,
  DEVICE_TYPE_POWER_SOURCE,
  DEVICE_TYPE_ROOT_NODE,
  DEVICE_TYPE_BRIDGED_NODE,
} from '../utils/deviceTree'
import type { EndpointTree } from '../types'

export interface ControllableEndpoint {
  nodeId: number
  deviceName: string
  endpointId: number
  clusters: number[]
  nodeLabel?: string
  deviceTypes: Array<{ id: number; name?: string }>
  productLabel?: string
}

export interface EndpointControlState {
  onOff?: boolean
  level?: number | null
  minLevel?: number
  maxLevel?: number
  colorMode?: number
  hue?: number
  saturation?: number
  colorTempMireds?: number
  colorTempMinMireds?: number
  colorTempMaxMireds?: number
  numberOfPositions?: number
  currentPosition?: number
  multiPressMax?: number
  occupancy?: boolean
  occupancySensorType?: string
  illuminanceMeasured?: number | null
  illuminanceMin?: number | null
  illuminanceMax?: number | null
  temperatureMeasured?: number | null
  temperatureMin?: number | null
  temperatureMax?: number | null
  loading: boolean
  error?: string
}

type StateKey = string

function stateKey(nodeId: number, endpointId: number): StateKey {
  return `${nodeId}:${endpointId}`
}

// Persist last-known endpoint structure so offline devices can still show placeholder cards.
const CACHE_KEY = 'matc-ui:control-cache:v1'

function loadEnumCache(): Map<number, ControllableEndpoint[]> {
  try {
    const raw = localStorage.getItem(CACHE_KEY)
    if (!raw) return new Map()
    const obj = JSON.parse(raw) as Record<string, ControllableEndpoint[]>
    return new Map(Object.entries(obj).map(([k, v]) => [Number(k), v]))
  } catch {
    return new Map()
  }
}

function saveEnumCache(map: Map<number, ControllableEndpoint[]>) {
  try {
    const obj: Record<string, ControllableEndpoint[]> = {}
    for (const [k, v] of map) obj[String(k)] = v
    localStorage.setItem(CACHE_KEY, JSON.stringify(obj))
  } catch {
    // localStorage may be unavailable in some environments
  }
}

let _statusWatcherInstalled = false
const _enumerateAttempts = new Map<number, number>()
const ENUMERATE_MAX_ATTEMPTS = 3
const ENUMERATE_BACKOFF_MS = 2000

export const useControlStore = defineStore('control', () => {
  const controllables = ref<ControllableEndpoint[]>([])
  const state = ref<Record<StateKey, EndpointControlState>>({})
  const loading = ref(false)
  // Tracks which enumerations are currently in flight to prevent concurrent duplicates.
  const enumerating = new Set<number>()
  // Tracks which nodes have been successfully enumerated from the device (not just from cache).
  const liveEnumerated = new Set<number>()

  async function enumerateDevice(device: DeviceDto) {
    console.log('[ctrl] enumerateDevice', device.node_id, '| enumerating:', [...enumerating], '| liveEnumerated:', [...liveEnumerated])
    if (enumerating.has(device.node_id)) {
      console.log('[ctrl] enumerateDevice', device.node_id, 'skipped: in-flight')
      return
    }

    // If already live-enumerated, cached entries are current structure - just refresh values.
    // Cache-only entries (liveEnumerated=false) still need a live enumerate to get real structure.
    if (liveEnumerated.has(device.node_id)) {
      const existing = controllables.value.filter(c => c.nodeId === device.node_id)
      if (existing.length > 0) {
        console.log('[ctrl] enumerateDevice', device.node_id, 'live-enumerated: refreshing state only')
        await Promise.all(existing.map(ep => refreshState(ep.nodeId, ep.endpointId, ep.clusters)))
        return
      }
    }

    console.log('[ctrl] enumerateDevice', device.node_id, 'starting full enumerate')
    enumerating.add(device.node_id)
    try {
      const [structure, deviceTreeRaw] = await Promise.all([
        invoke<EndpointTree>('read_endpoint_structure', { nodeId: device.node_id }),
        invoke<EndpointTree>('read_device_tree', { nodeId: device.node_id }).catch(() => null),
      ])

      const treeData = deviceTreeRaw ? buildDeviceTree(deviceTreeRaw) : null
      const parentOf = new Map<number, number>()
      if (treeData) {
        for (const [parent, children] of treeData.childrenOf) {
          for (const child of children) {
            parentOf.set(child, parent)
          }
        }
      }

      // Resolve device type names eagerly; filter out structural types not shown in UI.
      const skipTypes = new Set([DEVICE_TYPE_ROOT_NODE, DEVICE_TYPE_BRIDGED_NODE])
      const allTypeIds = new Set<number>()
      if (treeData) {
        for (const s of treeData.byEndpoint.values()) {
          for (const d of s.deviceTypes) {
            if (!skipTypes.has(d.id)) allTypeIds.add(d.id)
          }
        }
      }
      const dtNames = new Map<number, string>()
      await Promise.all([...allTypeIds].map(async id => {
        const name = await invoke<string | null>('get_device_type_name', { deviceType: id }).catch(() => null)
        if (name) dtNames.set(id, name)
      }))

      const newEntries: ControllableEndpoint[] = []

      for (const ep of structure.endpoints) {
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
        } else if (treeData) {
          const parentId = parentOf.get(ep.id)
          if (parentId != null) {
            const parentSummary = treeData.byEndpoint.get(parentId)
            if (parentSummary?.deviceTypes.some(d => d.id === DEVICE_TYPE_POWER_SOURCE)) {
              const label = parentSummary.label
              if (label && label.length > 0) nodeLabel = label
            }
          }
        }

        const epSummary = treeData?.byEndpoint.get(ep.id)
        const deviceTypes = (epSummary?.deviceTypes ?? [])
          .filter(d => !skipTypes.has(d.id))
          .map(d => ({ id: d.id, name: dtNames.get(d.id) }))
        const productLabel = treeData
          ? findInheritedProductLabel(ep.id, treeData.byEndpoint, parentOf)
          : undefined

        newEntries.push({
          nodeId: device.node_id,
          deviceName: device.name,
          endpointId: ep.id,
          clusters: presentClusters,
          nodeLabel,
          deviceTypes,
          productLabel,
        })
      }

      // Persist structure so the card is visible even when the device is later offline.
      const cache = loadEnumCache()
      if (newEntries.length > 0) {
        cache.set(device.node_id, newEntries)
        saveEnumCache(cache)
      }
      liveEnumerated.add(device.node_id)
      _enumerateAttempts.delete(device.node_id)
      console.log('[ctrl] enumerateDevice', device.node_id, 'done, newEntries:', newEntries.length)

      // Replace any stale entries for this node and append new ones.
      const otherEntries = controllables.value.filter(ep => ep.nodeId !== device.node_id)
      for (const ep of newEntries) {
        state.value[stateKey(ep.nodeId, ep.endpointId)] = { loading: true }
      }
      controllables.value = [...otherEntries, ...newEntries].sort((a, b) =>
        a.nodeId !== b.nodeId ? a.nodeId - b.nodeId : a.endpointId - b.endpointId
      )

      await runConcurrent(
        newEntries.map(ep => () => refreshState(ep.nodeId, ep.endpointId, ep.clusters)),
        3,
      )
    } catch (e) {
      const n = (_enumerateAttempts.get(device.node_id) ?? 0) + 1
      _enumerateAttempts.set(device.node_id, n)
      console.warn('[control] Failed to enumerate device', device.node_id, 'attempt', n, e)
    } finally {
      enumerating.delete(device.node_id)
      // If we finished (success or failure) but the node is connected and not enrolled,
      // a concurrent loadControllables wiped our entries mid-flight, or we failed outright.
      // Re-enumerate so the card eventually appears without requiring a manual page revisit.
      const nodeId = device.node_id
      const ds = useDevicesStore()
      if (
        !liveEnumerated.has(nodeId) &&
        !controllables.value.some(c => c.nodeId === nodeId) &&
        ds.deviceStatus[nodeId] === 'connected'
      ) {
        const n = _enumerateAttempts.get(nodeId) ?? 0
        _enumerateAttempts.set(nodeId, n + 1)
        if (n < ENUMERATE_MAX_ATTEMPTS) {
          const fresh = ds.devices.find(d => d.node_id === nodeId)
          if (fresh) {
            if (n === 0) {
              enumerateDevice(fresh)
            } else {
              setTimeout(() => enumerateDevice(fresh!), ENUMERATE_BACKOFF_MS * n)
            }
          }
        }
      }
    }
  }

  async function loadControllables() {
    loading.value = true
    try {
      const devicesStore = useDevicesStore()
      await devicesStore.fetchDevices()

      const validNodeIds = new Set(devicesStore.devices.map(d => d.node_id))
      const connectedIds = new Set(
        devicesStore.devices
          .filter(d => devicesStore.deviceStatus[d.node_id] === 'connected')
          .map(d => d.node_id)
      )
      console.log('[ctrl] loadControllables: devices:', [...validNodeIds], '| connected:', [...connectedIds], '| statuses:', JSON.stringify(devicesStore.deviceStatus))

      // Step 1: prune decommissioned devices.
      const removedIds = new Set<number>()
      controllables.value = controllables.value.filter(ep => {
        if (validNodeIds.has(ep.nodeId)) return true
        removedIds.add(ep.nodeId)
        return false
      })
      for (const key of Object.keys(state.value)) {
        if (removedIds.has(Number(key.split(':')[0]))) delete state.value[key]
      }
      for (const id of removedIds) liveEnumerated.delete(id)

      // Step 2: sync device names in case a device was renamed.
      const nameMap = new Map(devicesStore.devices.map(d => [d.node_id, d.name]))
      controllables.value = controllables.value.map(ep => {
        const newName = nameMap.get(ep.nodeId)
        return newName && newName !== ep.deviceName ? { ...ep, deviceName: newName } : ep
      })

      // Step 3: drop enrolled entries for currently-connected devices so we
      // re-enumerate fresh structure (picks up any firmware changes on Refresh).
      for (const ep of controllables.value.filter(c => connectedIds.has(c.nodeId))) {
        delete state.value[stateKey(ep.nodeId, ep.endpointId)]
      }
      controllables.value = controllables.value.filter(ep => !connectedIds.has(ep.nodeId))
      for (const id of connectedIds) liveEnumerated.delete(id)

      // Step 4: hydrate offline devices from cache so their cards remain visible.
      const cache = loadEnumCache()
      const enrolledIds = new Set(controllables.value.map(ep => ep.nodeId))
      for (const device of devicesStore.devices) {
        if (enrolledIds.has(device.node_id) || connectedIds.has(device.node_id)) continue
        const cached = cache.get(device.node_id)
        if (cached && cached.length > 0) {
          const updated = cached.map(ep => ({ ...ep, deviceName: device.name }))
          for (const ep of updated) {
            state.value[stateKey(ep.nodeId, ep.endpointId)] = { loading: false }
          }
          controllables.value = [...controllables.value, ...updated].sort((a, b) =>
            a.nodeId !== b.nodeId ? a.nodeId - b.nodeId : a.endpointId - b.endpointId
          )
        }
      }

      // Step 5: enumerate all connected devices to get fresh structure and state.
      await Promise.all(
        devicesStore.devices
          .filter(d => connectedIds.has(d.node_id))
          .map(device => enumerateDevice(device))
      )
    } finally {
      loading.value = false
    }
  }

  function _installStatusWatcher() {
    if (_statusWatcherInstalled) return
    _statusWatcherInstalled = true
    const devicesStore = useDevicesStore()

    // Track which nodes were connected last time the watcher fired.
    // Vue's deep watcher on a ref<Record> delivers the same proxy reference as
    // both newValue and oldValue for in-place mutations, so we cannot rely on
    // the oldValue argument to detect transitions.
    const lastSeenConnected = new Set<number>()

    watch(
      () => devicesStore.deviceStatus,
      (statusMap) => {
        console.log('[ctrl] watcher fired. statuses:', JSON.stringify(statusMap), '| liveEnumerated:', [...liveEnumerated], '| controllables:', controllables.value.map(c => c.nodeId))
        for (const [nodeIdStr, status] of Object.entries(statusMap)) {
          const nodeId = Number(nodeIdStr)
          const wasConnected = lastSeenConnected.has(nodeId)
          const isConnected = status === 'connected'
          if (isConnected) lastSeenConnected.add(nodeId)
          else lastSeenConnected.delete(nodeId)

          if (!isConnected) continue
          if (liveEnumerated.has(nodeId)) {
            // Device is already live-enumerated. Refresh state only on reconnect edge.
            if (!wasConnected) {
              console.log('[ctrl] watcher: node', nodeId, 'reconnected, refreshing state')
              const eps = controllables.value.filter(c => c.nodeId === nodeId)
              for (const ep of eps) refreshState(ep.nodeId, ep.endpointId, ep.clusters)
            }
            continue
          }
          // Connected but not yet live-enumerated: try (or retry) enumeration.
          console.log('[ctrl] watcher: node', nodeId, 'connected but not live-enumerated, calling enumerateDevice')
          const device = devicesStore.devices.find(d => d.node_id === nodeId)
          if (device) enumerateDevice(device)
          else console.warn('[ctrl] watcher: node', nodeId, 'not found in devices list!')
        }
      },
      { deep: true }
    )

    // Prune cards when a device is removed (devices list is replaced on removal).
    watch(
      () => devicesStore.devices,
      (newDevices) => {
        const validIds = new Set(newDevices.map(d => d.node_id))
        const removedIds = new Set<number>()
        controllables.value = controllables.value.filter(ep => {
          if (validIds.has(ep.nodeId)) return true
          removedIds.add(ep.nodeId)
          return false
        })
        if (removedIds.size > 0) {
          for (const key of Object.keys(state.value)) {
            if (removedIds.has(Number(key.split(':')[0]))) delete state.value[key]
          }
          for (const id of removedIds) {
            liveEnumerated.delete(id)
            lastSeenConnected.delete(id)
          }
          const cache = loadEnumCache()
          let changed = false
          for (const id of removedIds) {
            if (cache.delete(id)) changed = true
          }
          if (changed) saveEnumCache(cache)
        }
      }
    )
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
      // ColorMode is serialized as a Rust enum variant name string, not an integer.
      // Normalize to the numeric values used in the rest of the UI.
      let colorMode: number | null = null
      try {
        const raw = await readAttr(CLUSTER_ID_COLOR_CONTROL, ATTR_COLOR_MODE)
        const v = parseAttrJson<string | number>(raw)
        if (typeof v === 'number') {
          colorMode = v
        } else if (typeof v === 'string') {
          const nameToMode: Record<string, number> = {
            Currenthueandcurrentsaturation: 0,
            Currentxandcurrenty: 1,
            Colortemperaturemireds: 2,
          }
          colorMode = nameToMode[v] ?? null
        }
        if (colorMode != null) updates.colorMode = colorMode
      } catch { /* optional */ }

      try {
        const raw = await readAttr(CLUSTER_ID_COLOR_CONTROL, 0x0000)
        const hue = parseAttrJson<number>(raw)
        if (hue != null) updates.hue = hue
      } catch { /* CT-only lights */ }

      try {
        const raw = await readAttr(CLUSTER_ID_COLOR_CONTROL, 0x0001)
        const sat = parseAttrJson<number>(raw)
        if (sat != null) updates.saturation = sat
      } catch { /* same */ }

      try {
        const raw = await readAttr(CLUSTER_ID_COLOR_CONTROL, ATTR_COLOR_TEMP_MIREDS)
        const ct = parseAttrJson<number>(raw)
        if (ct != null && ct > 0) updates.colorTempMireds = ct
      } catch { /* HS-only lights */ }

      try {
        const raw = await readAttr(CLUSTER_ID_COLOR_CONTROL, ATTR_COLOR_TEMP_MIN_MIREDS)
        const v = parseAttrJson<number>(raw)
        if (v != null) updates.colorTempMinMireds = v
      } catch { /* optional */ }

      try {
        const raw = await readAttr(CLUSTER_ID_COLOR_CONTROL, ATTR_COLOR_TEMP_MAX_MIREDS)
        const v = parseAttrJson<number>(raw)
        if (v != null) updates.colorTempMaxMireds = v
      } catch { /* optional */ }
    }

    if (clusters.includes(CLUSTER_ID_SWITCH)) {
      try {
        const raw = await readAttr(CLUSTER_ID_SWITCH, ATTR_SWITCH_NUMBER_OF_POSITIONS)
        const v = parseAttrJson<number>(raw)
        if (v != null) updates.numberOfPositions = v
      } catch (e) {
        errors.push(String(e))
      }
      try {
        const raw = await readAttr(CLUSTER_ID_SWITCH, ATTR_SWITCH_CURRENT_POSITION)
        const v = parseAttrJson<number>(raw)
        if (v != null) updates.currentPosition = v
      } catch (e) {
        errors.push(String(e))
      }
      try {
        const raw = await readAttr(CLUSTER_ID_SWITCH, ATTR_SWITCH_MULTI_PRESS_MAX)
        const v = parseAttrJson<number>(raw)
        if (v != null) updates.multiPressMax = v
      } catch { /* optional attribute, ignore */ }
    }

    if (clusters.includes(CLUSTER_ID_OCCUPANCY_SENSING)) {
      try {
        const raw = await readAttr(CLUSTER_ID_OCCUPANCY_SENSING, ATTR_OCCUPANCY)
        const v = parseAttrJson<number>(raw)
        if (v != null) updates.occupancy = (v & 0x01) !== 0
      } catch (e) {
        errors.push(String(e))
      }
      try {
        const raw = await readAttr(CLUSTER_ID_OCCUPANCY_SENSING, ATTR_OCCUPANCY_SENSOR_TYPE)
        const v = parseAttrJson<string | number>(raw)
        if (v != null) {
          if (typeof v === 'string') {
            updates.occupancySensorType = v
          } else {
            const names: Record<number, string> = { 0: 'Pir', 1: 'Ultrasonic', 2: 'Pirandultrasonic', 3: 'Physicalcontact' }
            updates.occupancySensorType = names[v] ?? String(v)
          }
        }
      } catch { /* optional */ }
    }

    if (clusters.includes(CLUSTER_ID_ILLUMINANCE_MEASUREMENT)) {
      try {
        const raw = await readAttr(CLUSTER_ID_ILLUMINANCE_MEASUREMENT, ATTR_ILLUMINANCE_MEASURED_VALUE)
        updates.illuminanceMeasured = parseAttrJson<number | null>(raw)
      } catch (e) {
        errors.push(String(e))
      }
      try {
        const raw = await readAttr(CLUSTER_ID_ILLUMINANCE_MEASUREMENT, ATTR_ILLUMINANCE_MIN_MEASURED_VALUE)
        updates.illuminanceMin = parseAttrJson<number | null>(raw)
      } catch { /* optional */ }
      try {
        const raw = await readAttr(CLUSTER_ID_ILLUMINANCE_MEASUREMENT, ATTR_ILLUMINANCE_MAX_MEASURED_VALUE)
        updates.illuminanceMax = parseAttrJson<number | null>(raw)
      } catch { /* optional */ }
    }

    if (clusters.includes(CLUSTER_ID_TEMPERATURE_MEASUREMENT)) {
      try {
        const raw = await readAttr(CLUSTER_ID_TEMPERATURE_MEASUREMENT, ATTR_TEMPERATURE_MEASURED_VALUE)
        updates.temperatureMeasured = parseAttrJson<number | null>(raw)
      } catch (e) {
        errors.push(String(e))
      }
      try {
        const raw = await readAttr(CLUSTER_ID_TEMPERATURE_MEASUREMENT, ATTR_TEMPERATURE_MIN_MEASURED_VALUE)
        updates.temperatureMin = parseAttrJson<number | null>(raw)
      } catch { /* optional */ }
      try {
        const raw = await readAttr(CLUSTER_ID_TEMPERATURE_MEASUREMENT, ATTR_TEMPERATURE_MAX_MEASURED_VALUE)
        updates.temperatureMax = parseAttrJson<number | null>(raw)
      } catch { /* optional */ }
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
    await runConcurrent(
      controllables.value.map(ep => () => refreshState(ep.nodeId, ep.endpointId, ep.clusters)),
      3,
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

  async function setColorHueSat(nodeId: number, endpointId: number, hue: number, saturation: number) {
    const key = stateKey(nodeId, endpointId)
    state.value[key] = { ...(state.value[key] ?? { loading: false }), loading: true, error: undefined }
    try {
      await invoke('invoke_command_typed', {
        nodeId,
        endpoint: endpointId,
        cluster: CLUSTER_ID_COLOR_CONTROL,
        command: 0x06,
        args: { hue, saturation, transition_time: 0, options_mask: 0, options_override: 0 },
      })
      const rawH = await invoke<string>('read_single_attribute', {
        nodeId, endpoint: endpointId, cluster: CLUSTER_ID_COLOR_CONTROL, attrId: 0x0000,
      })
      const rawS = await invoke<string>('read_single_attribute', {
        nodeId, endpoint: endpointId, cluster: CLUSTER_ID_COLOR_CONTROL, attrId: 0x0001,
      })
      state.value[key] = {
        ...(state.value[key] ?? { loading: false }),
        hue: parseAttrJson<number>(rawH) ?? undefined,
        saturation: parseAttrJson<number>(rawS) ?? undefined,
        colorMode: 0,
        loading: false,
      }
    } catch (e) {
      state.value[key] = { ...(state.value[key] ?? { loading: false }), loading: false, error: String(e) }
    }
  }

  async function setColorTemperature(nodeId: number, endpointId: number, mireds: number) {
    const key = stateKey(nodeId, endpointId)
    state.value[key] = { ...(state.value[key] ?? { loading: false }), loading: true, error: undefined }
    try {
      await invoke('invoke_command_typed', {
        nodeId,
        endpoint: endpointId,
        cluster: CLUSTER_ID_COLOR_CONTROL,
        command: 0x0A,
        args: { color_temperature_mireds: mireds, transition_time: 0, options_mask: 0, options_override: 0 },
      })
      const raw = await invoke<string>('read_single_attribute', {
        nodeId, endpoint: endpointId, cluster: CLUSTER_ID_COLOR_CONTROL, attrId: ATTR_COLOR_TEMP_MIREDS,
      })
      state.value[key] = {
        ...(state.value[key] ?? { loading: false }),
        colorTempMireds: parseAttrJson<number>(raw) ?? undefined,
        colorMode: 2,
        loading: false,
      }
    } catch (e) {
      state.value[key] = { ...(state.value[key] ?? { loading: false }), loading: false, error: String(e) }
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

  _installStatusWatcher()

  return {
    controllables,
    state,
    loading,
    loadControllables,
    refreshEndpoint,
    refreshAll,
    toggleOnOff,
    setLevel,
    setColorHueSat,
    setColorTemperature,
    identify,
    getState,
  }
})
