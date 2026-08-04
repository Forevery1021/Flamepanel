import api from './client'
import type { Memo } from '@/types'

export function listMemos(kind?: string, done?: boolean) {
  const params: Record<string, string | boolean> = {}
  if (kind) params.kind = kind
  if (done !== undefined) params.done = done
  return api.get<Memo[]>('/memos', { params })
}

export function createMemo(content: string, kind = 'memo') {
  return api.post<Memo>('/memos', { content, kind })
}

export function updateMemo(id: number, data: { content?: string; done?: boolean }) {
  return api.put<Memo>(`/memos/${id}`, data)
}

export function deleteMemo(id: number) {
  return api.delete(`/memos/${id}`)
}
