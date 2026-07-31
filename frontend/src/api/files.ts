import api from './client'
import type { FileInfo } from '@/types'

export function listFiles(path: string) {
  return api.get<FileInfo[]>('/files', { params: { path } })
}

export function readFile(path: string) {
  return api.get<string>('/files/read', { params: { path } })
}

export function writeFile(path: string, content: string) {
  return api.post('/files/write', { path, content })
}

export function createFile(path: string) {
  return api.post('/files/create-file', { path })
}

export function createDir(path: string) {
  return api.post('/files/create-dir', { path })
}

export function deleteFile(path: string, recursive = false) {
  return api.delete('/files/delete', { params: { path, recursive } })
}

export function renameFile(oldPath: string, newPath: string) {
  return api.post('/files/rename', { old_path: oldPath, new_path: newPath })
}

export function chmodFile(path: string, mode: string) {
  return api.post('/files/chmod', { path, mode })
}

export function uploadFile(path: string, name: string, file: File) {
  const form = new FormData()
  form.append('file', file)
  return api.post(
    `/files/upload?path=${encodeURIComponent(path)}&name=${encodeURIComponent(name)}`,
    form,
  )
}

export function downloadFile(path: string) {
  return api.get(`/files/download?path=${encodeURIComponent(path)}`, { responseType: 'blob' })
}
