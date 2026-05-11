export const CLUSTER_ID_IDENTIFY = 0x0003
export const CLUSTER_ID_ON_OFF = 0x0006
export const CLUSTER_ID_LEVEL_CONTROL = 0x0008
export const CLUSTER_ID_COLOR_CONTROL = 0x0300
export const CLUSTER_ID_BRIDGED_DEVICE_BASIC_INFO = 0x0039
export const ATTR_NODE_LABEL = 0x0005
export const ATTR_COLOR_TEMP_MIREDS = 0x0007

export const CONTROLLABLE_CLUSTER_IDS = [
  CLUSTER_ID_ON_OFF,
  CLUSTER_ID_LEVEL_CONTROL,
  CLUSTER_ID_COLOR_CONTROL,
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

export function parseAttrJson<T>(value: string | null): T | null {
  if (value == null) return null
  try {
    return JSON.parse(value) as T
  } catch {
    return null
  }
}
