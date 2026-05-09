<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { useDiscoveryStore } from '../stores/discovery'
import { useDevicesStore } from '../stores/devices'
import type { DiscoveredDeviceDto, BleDeviceDto } from '../types'

const router = useRouter()
const discoveryStore = useDiscoveryStore()
const devicesStore = useDevicesStore()

// Pairing code tab state
type CodeTransport = 'network' | 'ble'
const codeForm = ref({
  pairing_code: '',
  name: '',
  node_id: 300,
  transport: 'network' as CodeTransport,
  wifi_ssid: '',
  wifi_password: '',
})
const codeLoading = ref(false)
const codeError = ref<string | null>(null)

const codeSubmitDisabled = computed(() => {
  if (!codeForm.value.pairing_code || !codeForm.value.name) return true
  if (codeForm.value.transport === 'ble' && !codeForm.value.wifi_ssid) return true
  return false
})

async function submitByCode() {
  codeLoading.value = true
  codeError.value = null
  try {
    const pairingCode = codeForm.value.pairing_code.trim()
    const name = codeForm.value.name.trim()
    const nodeId = codeForm.value.node_id

    if (codeForm.value.transport === 'ble') {
      await invoke('commission_ble', {
        pairingCode,
        nodeId,
        name,
        wifiSsid: codeForm.value.wifi_ssid,
        wifiPassword: codeForm.value.wifi_password,
      })
    } else {
      await invoke('commission_by_code', { pairingCode, nodeId, name })
    }
    await devicesStore.fetchDevices()
    router.push('/devices')
  } catch (e: any) {
    codeError.value = e?.toString() ?? 'Unknown error'
  } finally {
    codeLoading.value = false
  }
}

// mDNS / BLE scan state
const pairingCodeFilter = ref('')
const mdnsTimeout = ref(5)
const bleTimeout = ref(10)

const filteredMdns = computed(() => {
  const filter = pairingCodeFilter.value.trim()
  if (!filter) return discoveryStore.mdnsDevices
  return discoveryStore.mdnsDevices.filter(d =>
    d.discriminator?.includes(filter) ||
    d.instance?.toLowerCase().includes(filter.toLowerCase())
  )
})

// Commission dialog state (shared by mDNS and BLE)
const showCommissionDialog = ref(false)
const commissionTarget = ref<{ address?: string; addresses?: string[]; isBle?: boolean } | null>(null)
const commissionForm = ref({ name: '', node_id: 300, pin: 0, pairing_code: '', wifi_ssid: '', wifi_password: '' })
const commissioning = ref(false)
const commissionError = ref<string | null>(null)

function openCommission(device: DiscoveredDeviceDto) {
  const allAddresses = device.addresses.map(a => `${a}:${device.port}`)
  commissionTarget.value = { addresses: allAddresses, address: allAddresses[0], isBle: false }
  commissionForm.value = { name: device.name ?? '', node_id: 300, pin: 0, pairing_code: '', wifi_ssid: '', wifi_password: '' }
  commissionError.value = null
  showCommissionDialog.value = true
}

function openCommissionBle(device: BleDeviceDto) {
  commissionTarget.value = { isBle: true }
  commissionForm.value = { name: device.name ?? '', node_id: 300, pin: 0, pairing_code: '', wifi_ssid: '', wifi_password: '' }
  commissionError.value = null
  showCommissionDialog.value = true
}

async function doCommission() {
  commissioning.value = true
  commissionError.value = null
  try {
    const { name, node_id: nodeId, pin, wifi_ssid: wifiSsid, wifi_password: wifiPassword } = commissionForm.value
    const target = commissionTarget.value!

    if (target.isBle) {
      await invoke('commission_ble', {
        pairingCode: commissionForm.value.pairing_code,
        nodeId,
        name,
        wifiSsid,
        wifiPassword,
      })
    } else if (target.address && pin > 0) {
      await invoke('commission_by_address', { address: target.address, pin, nodeId, name })
    }

    showCommissionDialog.value = false
    await devicesStore.fetchDevices()
    router.push('/devices')
  } catch (e: any) {
    commissionError.value = e?.toString() ?? 'Unknown error'
  } finally {
    commissioning.value = false
  }
}
</script>

<template>
  <div class="view-container">
    <n-h2>Commission</n-h2>

    <n-tabs default-value="code" type="line">
      <!-- Pairing Code Tab -->
      <n-tab-pane name="code" tab="Pairing Code">
        <n-card style="max-width: 520px; margin-top: 12px">
          <n-form label-placement="top" @submit.prevent="submitByCode">
            <n-form-item label="Pairing Code" required>
              <n-input
                v-model:value="codeForm.pairing_code"
                placeholder="e.g. 0251-520-0076"
                :disabled="codeLoading"
              />
            </n-form-item>
            <n-form-item label="Device Name" required>
              <n-input
                v-model:value="codeForm.name"
                placeholder="e.g. kitchen light"
                :disabled="codeLoading"
              />
            </n-form-item>
            <n-form-item label="Node ID">
              <n-input-number
                v-model:value="codeForm.node_id"
                :min="1"
                style="width: 100%"
                :disabled="codeLoading"
              />
            </n-form-item>
            <n-form-item label="Transport">
              <n-radio-group v-model:value="codeForm.transport" :disabled="codeLoading">
                <n-radio-button value="network">Network (mDNS)</n-radio-button>
                <n-radio-button value="ble">BLE</n-radio-button>
              </n-radio-group>
            </n-form-item>

            <template v-if="codeForm.transport === 'ble'">
              <n-form-item label="Wi-Fi SSID" required>
                <n-input
                  v-model:value="codeForm.wifi_ssid"
                  placeholder="Home network"
                  :disabled="codeLoading"
                />
              </n-form-item>
              <n-form-item label="Wi-Fi Password">
                <n-input
                  v-model:value="codeForm.wifi_password"
                  type="password"
                  :disabled="codeLoading"
                />
              </n-form-item>
            </template>

            <n-text depth="3" style="font-size: 12px; display: block; margin-bottom: 16px">
              <template v-if="codeForm.transport === 'ble'">
                The device will be reached over Bluetooth and provisioned with the supplied Wi-Fi credentials.
                Make sure Bluetooth is enabled and the device is in commissioning mode.
              </template>
              <template v-else>
                The device will be discovered automatically on the network using the pairing code discriminator.
                Make sure the device is in commissioning mode.
              </template>
            </n-text>

            <n-button
              type="primary"
              attr-type="submit"
              :loading="codeLoading"
              :disabled="codeSubmitDisabled"
              block
            >
              {{ codeLoading ? 'Commissioning...' : 'Commission Device' }}
            </n-button>
          </n-form>

          <n-alert v-if="codeError" type="error" :title="codeError" style="margin-top: 16px" />
        </n-card>
      </n-tab-pane>

      <!-- mDNS Scan Tab -->
      <n-tab-pane name="mdns" tab="mDNS (Network)">
        <div class="controls">
          <n-input-number v-model:value="mdnsTimeout" :min="2" :max="30" style="width: 120px" size="small" />
          <n-text depth="3" style="margin: 0 8px">seconds</n-text>
          <n-button
            type="primary"
            :loading="discoveryStore.mdnsLoading"
            @click="discoveryStore.discoverMdns(mdnsTimeout)"
          >
            {{ discoveryStore.mdnsLoading ? 'Scanning...' : 'Start Discovery' }}
          </n-button>
          <n-input
            v-model:value="pairingCodeFilter"
            placeholder="Filter by discriminator..."
            clearable
            style="width: 200px; margin-left: auto"
            size="small"
          />
        </div>

        <n-alert v-if="discoveryStore.mdnsError" type="error" :title="discoveryStore.mdnsError" style="margin: 12px 0" />

        <n-spin :show="discoveryStore.mdnsLoading">
          <div v-if="!discoveryStore.mdnsLoading && filteredMdns.length === 0" class="empty-hint">
            <n-empty description="No commissionable devices found. Make sure the device is in commissioning mode." />
          </div>
          <n-list v-else bordered>
            <n-list-item v-for="dev in filteredMdns" :key="dev.instance">
              <n-thing :title="dev.name ?? dev.instance">
                <template #description>
                  <n-space size="small" wrap>
                    <n-tag v-if="dev.discriminator" size="small">Discriminator: {{ dev.discriminator }}</n-tag>
                    <n-tag v-if="dev.vendor_id" size="small" type="info">Vendor ID: {{ dev.vendor_id }}</n-tag>
                    <n-tag v-if="dev.product_id" size="small" type="info">Product ID: {{ dev.product_id }}</n-tag>
                    <n-tag v-for="addr in dev.addresses" :key="addr" size="small" type="default">
                      IP: {{ addr }}:{{ dev.port }}
                    </n-tag>
                  </n-space>
                </template>
                <template #action>
                  <n-button size="small" type="primary" @click="openCommission(dev)">Commission</n-button>
                </template>
              </n-thing>
            </n-list-item>
          </n-list>
        </n-spin>
      </n-tab-pane>

      <!-- BLE Scan Tab -->
      <n-tab-pane name="ble" tab="BLE">
        <div class="controls">
          <n-input-number v-model:value="bleTimeout" :min="2" :max="30" style="width: 120px" size="small" />
          <n-text depth="3" style="margin: 0 8px">seconds</n-text>
          <n-button
            type="primary"
            :loading="discoveryStore.bleLoading"
            @click="discoveryStore.scanBle(bleTimeout)"
          >
            {{ discoveryStore.bleLoading ? 'Scanning...' : 'Scan for BLE Devices' }}
          </n-button>
        </div>

        <n-alert v-if="discoveryStore.bleError" type="error" :title="discoveryStore.bleError" style="margin: 12px 0" />

        <n-spin :show="discoveryStore.bleLoading">
          <div v-if="!discoveryStore.bleLoading && discoveryStore.bleDevices.length === 0" class="empty-hint">
            <n-empty description="No BLE devices found. Make sure Bluetooth is enabled." />
          </div>
          <n-list v-else bordered>
            <n-list-item v-for="dev in discoveryStore.bleDevices" :key="dev.address">
              <n-thing :title="dev.name ?? 'BLE Device'">
                <template #description>
                  <n-space size="small" wrap>
                    <n-tag size="small">Discriminator: {{ dev.discriminator }}</n-tag>
                    <n-tag size="small" type="info">Vendor: {{ dev.vendor_id }}</n-tag>
                    <n-tag size="small" type="info">Product: {{ dev.product_id }}</n-tag>
                    <n-tag v-if="dev.cm_flag" size="small" type="success">Commissioning Open</n-tag>
                    <n-tag v-if="dev.rssi != null" size="small">RSSI: {{ dev.rssi }} dBm</n-tag>
                    <n-tag v-if="dev.tx_power != null" size="small">TX: {{ dev.tx_power }} dBm</n-tag>
                    <n-tag size="small" type="default">{{ dev.address }}</n-tag>
                  </n-space>
                </template>
                <template #action>
                  <n-button size="small" type="primary" @click="openCommissionBle(dev)">Commission via BLE</n-button>
                </template>
              </n-thing>
            </n-list-item>
          </n-list>
        </n-spin>
      </n-tab-pane>
    </n-tabs>

    <!-- Commission Dialog (mDNS and BLE) -->
    <n-modal v-model:show="showCommissionDialog" preset="dialog" title="Commission Device" style="width: 480px">
      <n-form label-placement="top">
        <n-form-item label="Device Name">
          <n-input v-model:value="commissionForm.name" placeholder="e.g. kitchen light" />
        </n-form-item>
        <n-form-item label="Node ID">
          <n-input-number v-model:value="commissionForm.node_id" :min="1" style="width: 100%" />
        </n-form-item>

        <template v-if="!commissionTarget?.isBle && commissionTarget?.address">
          <n-form-item label="PIN / Passcode">
            <n-input-number v-model:value="commissionForm.pin" :min="0" style="width: 100%" />
          </n-form-item>
          <n-form-item v-if="(commissionTarget.addresses?.length ?? 0) > 1" label="Address">
            <n-select
              v-model:value="commissionTarget.address"
              :options="commissionTarget.addresses!.map(a => ({ label: a, value: a }))"
            />
          </n-form-item>
          <n-text v-else depth="3" style="font-size: 12px">Address: {{ commissionTarget.address }}</n-text>
        </template>

        <template v-if="commissionTarget?.isBle">
          <n-form-item label="Pairing Code">
            <n-input v-model:value="commissionForm.pairing_code" placeholder="e.g. 0251-520-0076" />
          </n-form-item>
          <n-form-item label="Wi-Fi SSID">
            <n-input v-model:value="commissionForm.wifi_ssid" placeholder="Home network" />
          </n-form-item>
          <n-form-item label="Wi-Fi Password">
            <n-input v-model:value="commissionForm.wifi_password" type="password" />
          </n-form-item>
        </template>
      </n-form>

      <n-alert v-if="commissionError" type="error" :title="commissionError" style="margin-top: 12px" />

      <template #action>
        <n-space>
          <n-button @click="showCommissionDialog = false">Cancel</n-button>
          <n-button type="primary" :loading="commissioning" @click="doCommission">Commission</n-button>
        </n-space>
      </template>
    </n-modal>
  </div>
</template>

<style scoped>
.controls {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 16px;
  padding-top: 12px;
}

.empty-hint {
  padding: 40px 0;
}
</style>
