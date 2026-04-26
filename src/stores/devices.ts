import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { DeviceDto, DeviceInfoDto } from '../types'

export const useDevicesStore = defineStore('devices', () => {
  const devices = ref<DeviceDto[]>([])
  const deviceInfo = ref<Record<number, DeviceInfoDto>>({})
  const loading = ref(false)
  const loadingInfo = ref<Record<number, boolean>>({})

  async function fetchDevices() {
    loading.value = true
    try {
      devices.value = await invoke<DeviceDto[]>('list_devices')
    } finally {
      loading.value = false
    }
  }

  async function fetchDeviceInfo(nodeId: number): Promise<DeviceInfoDto> {
    loadingInfo.value[nodeId] = true
    try {
      const info = await invoke<DeviceInfoDto>('get_device_info', { nodeId })
      deviceInfo.value[nodeId] = info
      return info
    } finally {
      delete loadingInfo.value[nodeId]
    }
  }

  async function renameDevice(nodeId: number, name: string) {
    await invoke('rename_device', { nodeId, name })
    await fetchDevices()
  }

  async function removeDevice(nodeId: number) {
    await invoke('remove_device', { nodeId })
    delete deviceInfo.value[nodeId]
    await fetchDevices()
  }

  async function checkReachability(nodeId: number): Promise<boolean> {
    return await invoke<boolean>('check_reachability', { nodeId })
  }

  return { devices, deviceInfo, loading, loadingInfo, fetchDevices, fetchDeviceInfo, renameDevice, removeDevice, checkReachability }
})
