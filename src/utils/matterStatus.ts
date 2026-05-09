const MATTER_STATUS: Record<number, string> = {
  0x00: 'SUCCESS',
  0x01: 'FAILURE',
  0x7d: 'INVALID_SUBSCRIPTION',
  0x7e: 'UNSUPPORTED_ACCESS',
  0x7f: 'UNSUPPORTED_ENDPOINT',
  0x80: 'INVALID_ACTION',
  0x81: 'UNSUPPORTED_COMMAND',
  0x85: 'INVALID_COMMAND',
  0x86: 'UNSUPPORTED_ATTRIBUTE',
  0x87: 'CONSTRAINT_ERROR',
  0x88: 'UNSUPPORTED_WRITE',
  0x89: 'RESOURCE_EXHAUSTED',
  0x8c: 'UNREPORTABLE_ATTRIBUTE',
  0x8d: 'INVALID_DATA_TYPE',
  0x8f: 'UNSUPPORTED_READ',
  0x92: 'DATA_VERSION_MISMATCH',
  0x94: 'TIMEOUT',
  0x9c: 'BUSY',
  0x9d: 'ACCESS_RESTRICTED',
  0xc3: 'UNSUPPORTED_CLUSTER',
  0xc5: 'NO_UPSTREAM_SUBSCRIPTION',
}

function hex(n: number, pad = 2): string {
  return `0x${n.toString(16).toUpperCase().padStart(pad, '0')}`
}

export function formatStatus(code: number): string {
  const name = MATTER_STATUS[code]
  if (name) return `${name} (${hex(code)})`
  if (code >= 0x80 && code <= 0xbf) return `CLUSTER_SPECIFIC (${hex(code)})`
  return `STATUS_${hex(code)}`
}

export function isSuccess(code: number): boolean {
  return code === 0x00
}

export { MATTER_STATUS }
