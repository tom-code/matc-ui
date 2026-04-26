export interface DeviceDto {
  node_id: number
  name: string
  address: string
}

export interface DeviceInfoDto extends DeviceDto {
  vendor_name: string
  product_name: string
  sw_version: string
}

export interface DiscoveredDeviceDto {
  instance: string
  device: string
  addresses: string[]
  port: number
  discriminator: string | null
  vendor_id: string | null
  product_id: string | null
  name: string | null
}

export interface BleDeviceDto {
  discriminator: number
  vendor_id: number
  product_id: number
  cm_flag: boolean
  rssi: number | null
  name: string | null
  tx_power: number | null
  address: string
}

export interface AttrNode {
  id: number
  name: string
  value: string
}

export interface ClusterNode {
  id: number
  name: string
  attributes: AttrNode[]
}

export interface EndpointNode {
  id: number
  clusters: ClusterNode[]
}

export interface EndpointTree {
  endpoints: EndpointNode[]
}

export interface LogEntry {
  ts_ms: number
  level: string
  target: string
  message: string
}
