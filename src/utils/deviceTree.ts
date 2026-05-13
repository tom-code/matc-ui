import type { EndpointTree, EndpointNode } from '../types'

const CLUSTER_DESCRIPTOR = 0x001d
const ATTR_DEVICE_TYPE_LIST = 0x0000
const ATTR_PARTS_LIST = 0x0003

const CLUSTER_BASIC_INFORMATION = 0x0028
const CLUSTER_BRIDGED_DEVICE_BASIC_INFORMATION = 0x0039
const ATTR_VENDOR_NAME = 0x0001
const ATTR_PRODUCT_NAME = 0x0003
const ATTR_NODE_LABEL = 0x0005
const ATTR_PRODUCT_LABEL = 0x000e
const ATTR_REACHABLE = 0x0011

export const DEVICE_TYPE_AGGREGATOR = 0x000e
export const DEVICE_TYPE_BRIDGED_NODE = 0x0013
export const DEVICE_TYPE_ROOT_NODE = 0x0016
export const DEVICE_TYPE_POWER_SOURCE = 0x0011

export interface DeviceTypeDef {
  id: number
  revision: number
}

export interface EndpointSummary {
  endpoint: number
  deviceTypes: DeviceTypeDef[]
  parts: number[]
  label: string | null
  productName: string | null
  productLabel: string | null
  vendorName: string | null
  reachable: boolean | null
}

function findAttrValue(ep: EndpointNode, clusterId: number, attrId: number): string | null {
  const cluster = ep.clusters.find(c => c.id === clusterId)
  if (!cluster) return null
  const attr = cluster.attributes.find(a => a.id === attrId)
  return attr?.value ?? null
}

function parseJson<T>(raw: string | null): T | null {
  if (raw == null) return null
  try {
    return JSON.parse(raw) as T
  } catch {
    return null
  }
}

export function summarizeEndpoint(ep: EndpointNode): EndpointSummary {
  const rawDeviceTypes = parseJson<{ device_type: number; revision: number }[]>(
    findAttrValue(ep, CLUSTER_DESCRIPTOR, ATTR_DEVICE_TYPE_LIST)
  )
  const deviceTypes: DeviceTypeDef[] = (rawDeviceTypes ?? []).map(d => ({
    id: d.device_type ?? 0,
    revision: d.revision ?? 0,
  }))

  const parts = parseJson<number[]>(findAttrValue(ep, CLUSTER_DESCRIPTOR, ATTR_PARTS_LIST)) ?? []

  // prefer BridgedDeviceBasicInformation for bridged endpoints, fall back to BasicInformation
  const infoCluster = ep.clusters.find(c => c.id === CLUSTER_BRIDGED_DEVICE_BASIC_INFORMATION)
    ? CLUSTER_BRIDGED_DEVICE_BASIC_INFORMATION
    : CLUSTER_BASIC_INFORMATION

  const label = parseJson<string>(findAttrValue(ep, infoCluster, ATTR_NODE_LABEL))
  const productName = parseJson<string>(findAttrValue(ep, infoCluster, ATTR_PRODUCT_NAME))
  const productLabel = parseJson<string>(findAttrValue(ep, infoCluster, ATTR_PRODUCT_LABEL))
  const vendorName = parseJson<string>(findAttrValue(ep, infoCluster, ATTR_VENDOR_NAME))
  const reachable = parseJson<boolean>(findAttrValue(ep, infoCluster, ATTR_REACHABLE))

  return { endpoint: ep.id, deviceTypes, parts, label, productName, productLabel, vendorName, reachable }
}

// Returns the primary device type to display: skip Root Node and Bridged Node,
// use the first remaining type. Mirrors demo.rs::format_endpoint_summary.
export function primaryDeviceType(s: EndpointSummary): DeviceTypeDef | null {
  const skip = new Set([DEVICE_TYPE_ROOT_NODE, DEVICE_TYPE_BRIDGED_NODE])
  return s.deviceTypes.find(d => !skip.has(d.id)) ?? s.deviceTypes[0] ?? null
}

export function hasAggregator(tree: EndpointTree): boolean {
  return tree.endpoints.some(ep =>
    ep.clusters
      .find(c => c.id === CLUSTER_DESCRIPTOR)
      ?.attributes.find(a => a.id === ATTR_DEVICE_TYPE_LIST)
      ?.value != null &&
    (parseJson<{ device_type: number; revision: number }[]>(
      findAttrValue(ep, CLUSTER_DESCRIPTOR, ATTR_DEVICE_TYPE_LIST)
    ) ?? []).some(d => d.device_type === DEVICE_TYPE_AGGREGATOR)
  )
}

export interface DeviceTreeData {
  byEndpoint: Map<number, EndpointSummary>
  childrenOf: Map<number, number[]>
  roots: number[]
}

// Ports the direct-parent resolution from rust-matc/examples/demo.rs:808-833.
// Matter's PartsList per endpoint lists ALL ancestors (transitive), not just direct parent.
// The deepest ancestor is the one whose own PartsList contains the most of the
// endpoint's other candidate parents.
export function buildDeviceTree(tree: EndpointTree): DeviceTreeData {
  const byEndpoint = new Map<number, EndpointSummary>()
  for (const ep of tree.endpoints) {
    byEndpoint.set(ep.id, summarizeEndpoint(ep))
  }

  // allParents[child] = list of endpoints whose PartsList contains child
  const allParents = new Map<number, number[]>()
  for (const [epId, info] of byEndpoint) {
    for (const part of info.parts) {
      if (!allParents.has(part)) allParents.set(part, [])
      allParents.get(part)!.push(epId)
    }
  }

  // For each endpoint pick the deepest direct parent: among the candidate
  // parents, the one that has the most other candidate parents above it
  // (i.e. the most other candidates that contain p in their PartsList).
  const directParent = new Map<number, number>()
  for (const [ep, parents] of allParents) {
    let best: number | null = null
    let bestScore = -1
    for (const p of parents) {
      const score = parents.filter(q => q !== p && byEndpoint.get(q)?.parts.includes(p)).length
      if (score > bestScore) {
        bestScore = score
        best = p
      }
    }
    if (best != null) directParent.set(ep, best)
  }

  const childrenOf = new Map<number, number[]>()
  for (const [ep, parent] of directParent) {
    if (!childrenOf.has(parent)) childrenOf.set(parent, [])
    childrenOf.get(parent)!.push(ep)
  }
  for (const children of childrenOf.values()) {
    children.sort((a, b) => a - b)
  }

  const roots = [...byEndpoint.keys()]
    .filter(ep => !directParent.has(ep))
    .sort((a, b) => a - b)

  return { byEndpoint, childrenOf, roots }
}

// Walk from epId up the parentOf chain and return the first non-empty productLabel found.
export function findInheritedProductLabel(
  epId: number,
  byEndpoint: Map<number, EndpointSummary>,
  parentOf: Map<number, number>,
): string | undefined {
  const visited = new Set<number>()
  let cur: number | undefined = epId
  while (cur != null && !visited.has(cur)) {
    visited.add(cur)
    const s = byEndpoint.get(cur)
    if (s?.productLabel && s.productLabel.length > 0) return s.productLabel
    cur = parentOf.get(cur)
  }
  return undefined
}
