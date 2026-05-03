import type { DeviceConnectionStatus } from '../types'

export function statusTagType(s?: DeviceConnectionStatus): 'success' | 'error' | 'info' | 'default' {
  switch (s) {
    case 'connected': return 'success'
    case 'failed':    return 'error'
    case 'checking':  return 'info'
    default:          return 'default'
  }
}

export function statusLabel(s?: DeviceConnectionStatus): string {
  switch (s) {
    case 'connected': return 'Connected'
    case 'failed':    return 'Failed'
    case 'checking':  return 'Checking'
    default:          return 'Not checked'
  }
}
