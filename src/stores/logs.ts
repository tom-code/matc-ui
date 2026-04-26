import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { LogEntry } from '../types'

export type StoredEntry = LogEntry & { id: number }

let _seq = 0
let _subscribed = false

export const useLogsStore = defineStore('logs', () => {
  const entries = ref<StoredEntry[]>([])
  const paused = ref(false)
  const level = ref('debug')

  function push(entry: LogEntry) {
    if (entries.value.length >= 5000) {
      entries.value.shift()
    }
    entries.value.push({ ...entry, id: _seq++ })
  }

  async function loadRecent() {
    const data = await invoke<LogEntry[]>('get_recent_logs', { limit: 5000 })
    entries.value = data.map(e => ({ ...e, id: _seq++ }))
  }

  async function init() {
    level.value = await invoke<string>('get_log_level')
    await loadRecent()
    if (!_subscribed) {
      _subscribed = true
      await listen<LogEntry>('log://entry', event => {
        if (!paused.value) {
          push(event.payload)
        }
      })
    }
  }

  async function clear() {
    await invoke('clear_logs')
    entries.value = []
  }

  async function setLevel(next: string) {
    await invoke('set_log_level', { level: next })
    level.value = next
  }

  async function resume() {
    paused.value = false
    await loadRecent()
  }

  return { entries, paused, level, init, clear, setLevel, resume }
})
