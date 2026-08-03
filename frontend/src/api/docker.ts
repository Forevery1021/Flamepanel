import api from './client'
import type {
  ComposeProject,
  DockerContainer,
  DockerNetwork,
  DockerVolume,
  PruneResult,
} from '@/types'

export function listContainers(node_id?: number) {
  const params = node_id ? { node_id } : {}
  return api.get<DockerContainer[]>('/docker/containers', { params })
}

export function getContainer(id: string) {
  return api.get<DockerContainer>(`/docker/containers/${id}`)
}

export function inspectContainer(id: string) {
  return api.get(`/docker/containers/${id}/inspect`)
}

export function startContainer(id: string) {
  return api.post(`/docker/containers/${id}/start`)
}

export function stopContainer(id: string) {
  return api.post(`/docker/containers/${id}/stop`)
}

export function restartContainer(id: string) {
  return api.post(`/docker/containers/${id}/restart`)
}

export function removeContainer(id: string) {
  return api.post(`/docker/containers/${id}/remove`)
}

export function renameContainer(id: string, new_name: string) {
  return api.post(`/docker/containers/${id}/rename`, { new_name })
}

export function pauseContainer(id: string) {
  return api.post(`/docker/containers/${id}/pause`)
}

export function unpauseContainer(id: string) {
  return api.post(`/docker/containers/${id}/unpause`)
}

export function killContainer(id: string) {
  return api.post(`/docker/containers/${id}/kill`)
}

export function pruneContainers() {
  return api.post<PruneResult>('/docker/containers/prune')
}

export function containerLogs(id: string, tail = 100) {
  return api.get<string>(`/docker/containers/${id}/logs`, { params: { tail } })
}

export function containerStats(id: string) {
  return api.get(`/docker/containers/${id}/stats`)
}

export function listImages() {
  return api.get('/docker/images')
}

export function removeImage(id: string) {
  return api.post(`/docker/images/${id}/remove`)
}

export function pullImage(image: string) {
  return api.post('/docker/images/pull', { image })
}

export function tagImage(id: string, repo: string, tag: string) {
  return api.post(`/docker/images/${id}/tag`, { repo, tag })
}

export function pruneImages() {
  return api.post<PruneResult>('/docker/images/prune')
}

// ── 网络 ──
export function listNetworks() {
  return api.get<DockerNetwork[]>('/docker/networks')
}

export function createNetwork(name: string, driver = 'bridge', subnet?: string) {
  return api.post('/docker/networks', { name, driver, subnet })
}

export function removeNetwork(id: string) {
  return api.delete(`/docker/networks/${id}`)
}

export function connectNetwork(networkId: string, containerId: string) {
  return api.post(`/docker/networks/${networkId}/connect`, { container_id: containerId })
}

export function disconnectNetwork(networkId: string, containerId: string, force = false) {
  return api.post(`/docker/networks/${networkId}/disconnect`, {
    container_id: containerId,
    force,
  })
}

export function pruneNetworks() {
  return api.post<PruneResult>('/docker/networks/prune')
}

// ── 卷 ──
export function listVolumes() {
  return api.get<DockerVolume[]>('/docker/volumes')
}

export function createVolume(name: string, driver = 'local') {
  return api.post('/docker/volumes', { name, driver })
}

export function removeVolume(name: string, force = false) {
  return api.delete(`/docker/volumes/${name}`, { params: { force } })
}

export function pruneVolumes() {
  return api.post<PruneResult>('/docker/volumes/prune')
}

// ── Compose ──
export function listComposeProjects() {
  return api.get<ComposeProject[]>('/docker/compose')
}

export function composeDeploy(project_name: string, compose_yaml: string) {
  return api.post('/docker/compose/deploy', { project_name, compose_yaml })
}

export function composeUp(project_name: string) {
  return api.post(`/docker/compose/${project_name}/up`)
}

export function composeDown(project_name: string) {
  return api.post(`/docker/compose/${project_name}/down`)
}
