<script setup lang="ts">
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

const props = defineProps<{ nodeId: number }>()

const endpoint = ref(1)
const cluster = ref('')
const command = ref('')
const payloadHex = ref('')
const loading = ref(false)
const result = ref<string | null>(null)
const error = ref<string | null>(null)

function parseHexOrDec(s: string): number {
  s = s.trim()
  if (s.startsWith('0x') || s.startsWith('0X')) {
    return parseInt(s.slice(2), 16)
  }
  return parseInt(s, 10)
}

async function send() {
  loading.value = true
  result.value = null
  error.value = null
  try {
    const clusterId = parseHexOrDec(cluster.value)
    const commandId = parseHexOrDec(command.value)
    const res = await invoke<string>('invoke_command', {
      nodeId: props.nodeId,
      endpoint: endpoint.value,
      cluster: clusterId,
      command: commandId,
      payloadHex: payloadHex.value.trim(),
    })
    result.value = res
  } catch (e: any) {
    error.value = e?.toString() ?? 'Unknown error'
  } finally {
    loading.value = false
  }
}

// Common presets for quick access
const presets = [
  { label: 'OnOff - On', endpoint: 1, cluster: '0x0006', command: '0x01', payload: '' },
  { label: 'OnOff - Off', endpoint: 1, cluster: '0x0006', command: '0x00', payload: '' },
  { label: 'OnOff - Toggle', endpoint: 1, cluster: '0x0006', command: '0x02', payload: '' },
]

function applyPreset(p: typeof presets[0]) {
  endpoint.value = p.endpoint
  cluster.value = p.cluster
  command.value = p.command
  payloadHex.value = p.payload
}
</script>

<template>
  <div>
    <n-space vertical size="small">
      <div>
        <n-text strong style="font-size: 12px">Quick Presets</n-text>
        <n-space size="small" style="margin-top: 4px" wrap>
          <n-button
            v-for="p in presets"
            :key="p.label"
            size="tiny"
            @click="applyPreset(p)"
          >
            {{ p.label }}
          </n-button>
        </n-space>
      </div>

      <n-divider style="margin: 8px 0" />

      <n-form label-placement="left" label-width="80" size="small">
        <n-form-item label="Endpoint">
          <n-input-number v-model:value="endpoint" :min="0" style="width: 100%" />
        </n-form-item>
        <n-form-item label="Cluster">
          <n-input v-model:value="cluster" placeholder="0x0006 or 6" />
        </n-form-item>
        <n-form-item label="Command">
          <n-input v-model:value="command" placeholder="0x01 or 1" />
        </n-form-item>
        <n-form-item label="Payload (hex)">
          <n-input v-model:value="payloadHex" placeholder="optional hex bytes" />
        </n-form-item>
      </n-form>

      <n-button
        type="primary"
        block
        :loading="loading"
        :disabled="!cluster || !command"
        @click="send"
      >
        Send Command
      </n-button>

      <n-alert v-if="error" type="error" :title="error" />
      <div v-if="result" class="result-box">
        <n-text strong style="font-size: 12px">Response:</n-text>
        <pre class="result-pre">{{ result }}</pre>
      </div>
    </n-space>
  </div>
</template>

<style scoped>
.result-box {
  margin-top: 8px;
}

.result-pre {
  font-size: 11px;
  background: var(--n-code-color);
  border-radius: 4px;
  padding: 8px;
  overflow: auto;
  max-height: 200px;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
