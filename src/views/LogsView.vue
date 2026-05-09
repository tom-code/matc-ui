<script setup lang="ts">
import { h, ref, computed, watch, nextTick, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { NTag } from 'naive-ui'
import type { DataTableColumns } from 'naive-ui'
import { useLogsStore, type StoredEntry } from '../stores/logs'

const store = useLogsStore()

const stdoutLogging = ref(false)

async function toggleStdout(val: boolean) {
  stdoutLogging.value = val
  await invoke('set_stdout_logging', { enabled: val })
}

const tableRef = ref()
const textFilter = ref('')
const autoScroll = ref(true)

const levelOptions = [
  { label: 'Off', value: 'off' },
  { label: 'Error', value: 'error' },
  { label: 'Warn', value: 'warn' },
  { label: 'Info', value: 'info' },
  { label: 'Debug', value: 'debug' },
  { label: 'Trace', value: 'trace' },
]

function formatTime(tsMs: number): string {
  const d = new Date(tsMs)
  const hh = String(d.getHours()).padStart(2, '0')
  const mm = String(d.getMinutes()).padStart(2, '0')
  const ss = String(d.getSeconds()).padStart(2, '0')
  const ms = String(d.getMilliseconds()).padStart(3, '0')
  return `${hh}:${mm}:${ss}.${ms}`
}

function levelTagType(level: string): 'default' | 'info' | 'warning' | 'error' {
  switch (level) {
    case 'ERROR': return 'error'
    case 'WARN': return 'warning'
    case 'INFO': return 'info'
    default: return 'default'
  }
}

const columns: DataTableColumns<StoredEntry> = [
  {
    title: 'Time',
    key: 'ts_ms',
    width: 110,
    render: row => formatTime(row.ts_ms),
  },
  {
    title: 'Level',
    key: 'level',
    width: 80,
    render: row =>
      h(NTag, { type: levelTagType(row.level), size: 'small', style: 'font-size: 11px; font-family: monospace' }, { default: () => row.level }),
  },
  {
    title: 'Target',
    key: 'target',
    width: 240,
    ellipsis: { tooltip: true },
    render: row => h('span', { style: 'font-family: monospace; font-size: 12px' }, row.target),
  },
  {
    title: 'Message',
    key: 'message',
    ellipsis: { tooltip: true },
    render: row => h('span', { style: 'font-family: monospace; font-size: 12px' }, row.message),
  },
]

const filteredEntries = computed<StoredEntry[]>(() => {
  const filter = textFilter.value.trim().toLowerCase()
  if (!filter) return store.entries
  return store.entries.filter(
    e => e.target.toLowerCase().includes(filter) || e.message.toLowerCase().includes(filter)
  )
})

const tableHeight = computed(() => window.innerHeight - 180)

const entryCount = computed(() => store.entries.length)
watch(entryCount, () => {
  if (autoScroll.value && !store.paused) {
    nextTick(() => {
      tableRef.value?.scrollTo({ behavior: 'auto', top: Number.MAX_SAFE_INTEGER })
    })
  }
})

onMounted(async () => {
  store.init()
  stdoutLogging.value = await invoke<boolean>('get_stdout_logging')
})
</script>

<template>
  <div class="logs-view">
    <div class="toolbar">
      <n-space align="center" wrap>
        <n-select
          :value="store.level"
          :options="levelOptions"
          size="small"
          style="width: 100px"
          @update:value="(v: string) => store.setLevel(v)"
        />
        <n-input
          v-model:value="textFilter"
          placeholder="Filter target / message"
          size="small"
          clearable
          style="width: 240px"
        />
        <n-switch v-model:value="autoScroll" size="small" />
        <n-text depth="3" style="font-size: 13px">Auto-scroll</n-text>
        <n-switch :value="stdoutLogging" size="small" @update:value="toggleStdout" />
        <n-text depth="3" style="font-size: 13px">Stdout</n-text>
        <n-button
          size="small"
          :type="store.paused ? 'primary' : 'default'"
          @click="store.paused ? store.resume() : (store.paused = true)"
        >
          {{ store.paused ? 'Resume' : 'Pause' }}
        </n-button>
        <n-button size="small" @click="store.clear()">Clear</n-button>
        <n-text depth="3" style="font-size: 13px">{{ filteredEntries.length }} entries</n-text>
      </n-space>
    </div>
    <n-data-table
      ref="tableRef"
      :columns="columns"
      :data="filteredEntries"
      :row-key="(row: StoredEntry) => row.id"
      :virtual-scroll="true"
      :max-height="tableHeight"
      size="small"
    />
  </div>
</template>

<style scoped>
.logs-view {
  height: 100%;
  display: flex;
  flex-direction: column;
}
.toolbar {
  flex-shrink: 0;
  margin-bottom: 12px;
}
</style>
