import { describe, it, expect, vi, beforeEach } from 'vitest'

// 冒烟：项目构建器决策 API 层（M-B 主线二 Stage A-8 · Doc/20 §5.8 ②）
vi.mock('@/api/client', () => {
  const get = vi.fn()
  return { default: { get } }
})

import api from '@/api/client'
import { getProjectBuilder } from '@/api/projects'

describe('projectBuilder API（构建器自动协商）', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('getProjectBuilder 走 GET /projects/:id/builder', async () => {
    ;(api.get as ReturnType<typeof vi.fn>).mockResolvedValue({
      data: { project_id: 5, build_type: 'dockerfile', image: 'flame-proj/app:5' },
    })
    const res = await getProjectBuilder(5)
    expect(api.get).toHaveBeenCalledWith('/projects/5/builder')
    expect(res.data.build_type).toBe('dockerfile')
    expect(res.data.image).toBe('flame-proj/app:5')
  })

  it('getProjectBuilder runtime 决策无 image 字段', async () => {
    ;(api.get as ReturnType<typeof vi.fn>).mockResolvedValue({
      data: { project_id: 6, build_type: 'runtime' },
    })
    const res = await getProjectBuilder(6)
    expect(res.data.build_type).toBe('runtime')
    expect(res.data.image).toBeUndefined()
  })
})
