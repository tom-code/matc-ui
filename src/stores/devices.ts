import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { DeviceDto, DeviceInfoDto, DeviceConnectionStatus, DeviceStatusDto } from '../types'

let _subscribed = false

export const useDevicesStore = defineStore('devices', () => {
  const devices = ref<DeviceDto[]>([])
  const deviceInfo = ref<Record<number, DeviceInfoDto>>({})
  const loading = ref(false)
  const deviceStatus = ref<Record<number, DeviceConnectionStatus>>({})
  const statusError = ref<Record<number, string>>({})

  async function fetchDevices() {
    loading.value = true
    try {
      devices.value = await invoke<DeviceDto[]>('list_devices')
    } finally {
      loading.value = false
    }
  }

  function applyStatusEvent(s: DeviceStatusDto) {
    if (s.status === 'removed') {
      delete deviceStatus.value[s.node_id]
      delete statusError.value[s.node_id]
      delete deviceInfo.value[s.node_id]
      devices.value = devices.value.filter(d => d.node_id !== s.node_id)
      return
    }
    deviceStatus.value[s.node_id] = s.status as DeviceConnectionStatus
    if (s.error) {
      statusError.value[s.node_id] = s.error
    } else {
      delete statusError.value[s.node_id]
    }
    if (s.info) {
      deviceInfo.value[s.node_id] = s.info
    }
  }

  async function init() {
    if (_subscribed) return
    _subscribed = true
    const seenFromEvents = new Set<number>()
    console.log('[dev] init: registering listener')
    await listen<DeviceStatusDto>('device://status', event => {
      const p = event.payload
      console.log('[dev] event:', p.node_id, p.status)
      seenFromEvents.add(p.node_id)
      applyStatusEvent(p)
    })
    console.log('[dev] init: listener registered, fetching devices')
    await fetchDevices()
    console.log('[dev] init: fetched', devices.value.length, 'devices, taking snapshot')
    const snap = await invoke<DeviceStatusDto[]>('get_device_statuses')
    console.log('[dev] init: snapshot:', snap.map(s => `${s.node_id}=${s.status}`).join(', '))
    for (const s of snap) {
      if (seenFromEvents.has(s.node_id)) {
        console.log('[dev] init: skip snapshot for', s.node_id, '(already from event)')
        continue
      }
      applyStatusEvent(s)
    }
    console.log('[dev] init: done. deviceStatus:', JSON.stringify(deviceStatus.value))
  }

  async function fetchDeviceInfo(nodeId: number, forceRefresh = false): Promise<void> {
    // Triggers a backend probe; the result arrives via the device://status event.
    await invoke('get_device_info', { nodeId, forceRefresh }).catch(() => {})
  }

  async function renameDevice(nodeId: number, name: string) {
    await invoke('rename_device', { nodeId, name })
    await fetchDevices()
  }

  async function removeDevice(nodeId: number) {
    await invoke('remove_device', { nodeId })
    // Eagerly prune local state; the backend also emits a 'removed' event.
    delete deviceStatus.value[nodeId]
    delete statusError.value[nodeId]
    delete deviceInfo.value[nodeId]
    await fetchDevices()
  }

  return {
    devices, deviceInfo, loading,
    deviceStatus, statusError,
    init, fetchDevices, fetchDeviceInfo, renameDevice, removeDevice,
  }
})
