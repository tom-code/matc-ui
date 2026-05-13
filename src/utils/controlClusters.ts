export const CLUSTER_ID_IDENTIFY = 0x0003
export const CLUSTER_ID_ON_OFF = 0x0006
export const CLUSTER_ID_LEVEL_CONTROL = 0x0008
export const CLUSTER_ID_COLOR_CONTROL = 0x0300
export const CLUSTER_ID_SWITCH = 0x003B
export const CLUSTER_ID_ILLUMINANCE_MEASUREMENT = 0x0400
export const CLUSTER_ID_TEMPERATURE_MEASUREMENT = 0x0402
export const CLUSTER_ID_OCCUPANCY_SENSING = 0x0406
export const CLUSTER_ID_BRIDGED_DEVICE_BASIC_INFO = 0x0039
export const ATTR_NODE_LABEL = 0x0005
export const ATTR_PRODUCT_LABEL = 0x000E
export const ATTR_COLOR_MODE = 0x0008
export const ATTR_COLOR_TEMP_MIREDS = 0x0007
export const ATTR_COLOR_TEMP_MIN_MIREDS = 0x400B
export const ATTR_COLOR_TEMP_MAX_MIREDS = 0x400C
export const ATTR_SWITCH_NUMBER_OF_POSITIONS = 0x0000
export const ATTR_SWITCH_CURRENT_POSITION = 0x0001
export const ATTR_SWITCH_MULTI_PRESS_MAX = 0x0002
export const ATTR_OCCUPANCY = 0x0000
export const ATTR_OCCUPANCY_SENSOR_TYPE = 0x0001
export const ATTR_ILLUMINANCE_MEASURED_VALUE = 0x0000
export const ATTR_ILLUMINANCE_MIN_MEASURED_VALUE = 0x0001
export const ATTR_ILLUMINANCE_MAX_MEASURED_VALUE = 0x0002
export const ATTR_TEMPERATURE_MEASURED_VALUE = 0x0000
export const ATTR_TEMPERATURE_MIN_MEASURED_VALUE = 0x0001
export const ATTR_TEMPERATURE_MAX_MEASURED_VALUE = 0x0002

export const CONTROLLABLE_CLUSTER_IDS = [
  CLUSTER_ID_ON_OFF,
  CLUSTER_ID_LEVEL_CONTROL,
  CLUSTER_ID_COLOR_CONTROL,
  CLUSTER_ID_SWITCH,
  CLUSTER_ID_ILLUMINANCE_MEASUREMENT,
  CLUSTER_ID_TEMPERATURE_MEASUREMENT,
  CLUSTER_ID_OCCUPANCY_SENSING,
]

// Tanner Helland's algorithm: kelvin (1000-40000) -> [r, g, b] each 0-255
export function kelvinToRgb(kelvin: number): [number, number, number] {
  const t = Math.max(1000, Math.min(40000, kelvin)) / 100
  const clamp = (v: number) => Math.min(255, Math.max(0, Math.round(v)))

  const r = t <= 66
    ? 255
    : clamp(329.698727446 * Math.pow(t - 60, -0.1332047592))

  const g = t <= 66
    ? clamp(99.4708025861 * Math.log(t) - 161.1195681661)
    : clamp(288.1221695283 * Math.pow(t - 60, -0.0755148492))

  const b = t >= 66
    ? 255
    : t <= 19
      ? 0
      : clamp(138.5177312231 * Math.log(t - 10) - 305.0447927307)

  return [r, g, b]
}

// Matter spec: raw = 10000 * log10(lux) + 1. Returns null for invalid (0xFFFF or null input).
// Returns 0 when raw === 0 (below measurement range).
export function luxFromRaw(raw: number | null): number | null {
  if (raw == null || raw === 0xFFFF) return null
  if (raw === 0) return 0
  return Math.pow(10, (raw - 1) / 10000)
}

// Matter spec: raw is signed int16 in units of 0.01 C.
// Returns null for invalid (0x8000 = -32768 sentinel, or null input).
export function tempCFromRaw(raw: number | null): number | null {
  if (raw == null || raw === -32768) return null
  return raw / 100
}

// Matter hue 0-254 (0-360 deg) + saturation 0-254 (0-100%) -> #rrggbb using HSV value=1
export function matterHueSatToHex(hue: number, sat: number): string {
  const h = (hue / 254) * 360
  const s = sat / 254
  const c = s
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1))
  const m = 1 - c
  let r = 0, g = 0, b = 0
  if (h < 60) { r = c; g = x }
  else if (h < 120) { r = x; g = c }
  else if (h < 180) { g = c; b = x }
  else if (h < 240) { g = x; b = c }
  else if (h < 300) { r = x; b = c }
  else { r = c; b = x }
  const ri = Math.round((r + m) * 255)
  const gi = Math.round((g + m) * 255)
  const bi = Math.round((b + m) * 255)
  return '#' + [ri, gi, bi].map(v => v.toString(16).padStart(2, '0')).join('')
}

// #rrggbb -> Matter [hue 0-254, saturation 0-254] via RGB->HSV
export function rgbHexToMatterHueSat(hex: string): [number, number] {
  const r = parseInt(hex.slice(1, 3), 16) / 255
  const g = parseInt(hex.slice(3, 5), 16) / 255
  const b = parseInt(hex.slice(5, 7), 16) / 255
  const max = Math.max(r, g, b)
  const min = Math.min(r, g, b)
  const d = max - min
  let h = 0
  if (d !== 0) {
    if (max === r) h = ((g - b) / d + 6) % 6
    else if (max === g) h = (b - r) / d + 2
    else h = (r - g) / d + 4
    h *= 60
  }
  const s = max === 0 ? 0 : d / max
  return [Math.round((h / 360) * 254), Math.round(s * 254)]
}

export function parseAttrJson<T>(value: string | null): T | null {
  if (value == null) return null
  try {
    return JSON.parse(value) as T
  } catch {
    return null
  }
}
