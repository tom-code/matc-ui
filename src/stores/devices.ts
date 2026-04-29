import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { DeviceDto, DeviceInfoDto, DeviceConnectionStatus } from '../types'

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

  async function fetchDeviceInfo(nodeId: number): Promise<DeviceInfoDto> {
    deviceStatus.value[nodeId] = 'checking'
    try {
      const info = await invoke<DeviceInfoDto>('get_device_info', { nodeId })
      deviceInfo.value[nodeId] = info
      deviceStatus.value[nodeId] = 'connected'
      delete statusError.value[nodeId]
      return info
    } catch (e) {
      deviceStatus.value[nodeId] = 'failed'
      statusError.value[nodeId] = String(e)
      throw e
    }
  }

  async function renameDevice(nodeId: number, name: string) {
    await invoke('rename_device', { nodeId, name })
    await fetchDevices()
  }

  async function removeDevice(nodeId: number) {
    await invoke('remove_device', { nodeId })
    delete deviceInfo.value[nodeId]
    delete deviceStatus.value[nodeId]
    delete statusError.value[nodeId]
    await fetchDevices()
  }

  async function probeAllDevices(): Promise<void> {
    await Promise.all(
      devices.value.map(d => fetchDeviceInfo(d.node_id).catch(() => undefined))
    )
  }

  return {
    devices, deviceInfo, loading,
    deviceStatus, statusError,
    fetchDevices, fetchDeviceInfo, renameDevice, removeDevice,
    probeAllDevices,
  }
})
