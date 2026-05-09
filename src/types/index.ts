export type DeviceConnectionStatus = 'unknown' | 'checking' | 'connected' | 'failed'

export interface DeviceDto {
  node_id: number
  name: string
  address: string
}

export interface DeviceInfoDto extends DeviceDto {
  vendor_name: string
  product_name: string
  hw_version: string
  sw_version: string
  has_aggregator: boolean
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
  value: string | null
  error: string | null
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

export interface AttrProgressEvent {
  phase: 'connecting' | 'discover' | 'read' | 'done'
  endpointIndex: number
  endpointCount: number
  endpointId: number | null
  endpointAttrIndex: number
  endpointAttrTotal: number
  currentCluster: string | null
}

export interface LogEntry {
  ts_ms: number
  level: string
  target: string
  message: string
}

// --- Command schema types (from Tauri clusters commands) ---

export interface CommandDto {
  id: number
  name: string
}

export interface AttributeDto {
  id: number
  name: string
}

// Discriminated union matching the Rust FieldKind serde shape.
export type FieldKind =
  | { type: 'u8' | 'u16' | 'u32' | 'u64' | 'i8' | 'i16' | 'i32' | 'i64' | 'bool' | 'string' | 'octet_string' }
  | { type: 'enum'; name: string; variants: [number, string][] }
  | { type: 'bitmap'; name: string; bits: [number, string][] }
  | { type: 'struct'; name: string }
  | { type: 'list'; entry_type: string }

export interface CommandFieldDto {
  tag: number
  name: string
  kind: FieldKind
  optional: boolean
  nullable: boolean
}

export interface CommandSchemaDto {
  fields: CommandFieldDto[]
}

export type InvokeResult =
  | { kind: 'status'; code: number }
  | { kind: 'data'; tlv: string }

export interface OpenCommissioningWindowResult {
  status: number
  manual_pairing_code: string
  pin: number
  discriminator: number
  iterations: number
  timeout_secs: number
}
