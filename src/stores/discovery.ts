import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { DiscoveredDeviceDto, BleDeviceDto } from '../types'

export const useDiscoveryStore = defineStore('discovery', () => {
  const mdnsDevices = ref<DiscoveredDeviceDto[]>([])
  const bleDevices = ref<BleDeviceDto[]>([])
  const mdnsLoading = ref(false)
  const bleLoading = ref(false)
  const mdnsError = ref<string | null>(null)
  const bleError = ref<string | null>(null)

  async function discoverMdns(timeoutSecs = 5) {
    mdnsLoading.value = true
    mdnsError.value = null
    mdnsDevices.value = []
    try {
      mdnsDevices.value = await invoke<DiscoveredDeviceDto[]>('discover_mdns', {
        timeoutSecs,
      })
    } catch (e: any) {
      mdnsError.value = e?.toString() ?? 'Unknown error'
    } finally {
      mdnsLoading.value = false
    }
  }

  async function scanBle(timeoutSecs = 10) {
    bleLoading.value = true
    bleError.value = null
    bleDevices.value = []
    try {
      bleDevices.value = await invoke<BleDeviceDto[]>('scan_ble', {
        timeoutSecs,
      })
    } catch (e: any) {
      bleError.value = e?.toString() ?? 'Unknown error'
    } finally {
      bleLoading.value = false
    }
  }

  return { mdnsDevices, bleDevices, mdnsLoading, bleLoading, mdnsError, bleError, discoverMdns, scanBle }
})
