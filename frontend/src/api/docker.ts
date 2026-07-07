import api from './client'
import type { DockerContainer } from '@/types'

export function listContainers(node_id?: number) {
  const params = node_id ? { node_id } : {}
  return api.get<DockerContainer[]>('/docker/containers', { params })
}

export function getContainer(id: string) {
  return api.get<DockerContainer>(`/docker/containers/${id}`)
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

export function composeDeploy(project_name: string, compose_yaml: string) {
  return api.post('/docker/compose/deploy', { project_name, compose_yaml })
}

export function composeUp(project_name: string) {
  return api.post(`/docker/compose/${project_name}/up`)
}

export function composeDown(project_name: string) {
  return api.post(`/docker/compose/${project_name}/down`)
}
