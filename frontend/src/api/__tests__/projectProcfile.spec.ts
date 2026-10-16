import { describe, it, expect, vi, beforeEach } from 'vitest'

// 冒烟：项目 Procfile 进程声明 API 层（M-B 主线二 Stage A-8 · Doc/20 §5.8 ③）
vi.mock('@/api/client', () => {
  const get = vi.fn()
  return { default: { get } }
})

import api from '@/api/client'
import { getProjectProcfile } from '@/api/projects'

describe('projectProcfile API（Procfile 进程模型）', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('getProjectProcfile 走 GET /projects/:id/procfile', async () => {
    ;(api.get as ReturnType<typeof vi.fn>).mockResolvedValue({
      data: {
        project_id: 5,
        processes: [
          { proc_type: 'web', command: 'node server.js' },
          { proc_type: 'worker', command: 'node worker.js' },
        ],
      },
    })
    const res = await getProjectProcfile(5)
    expect(api.get).toHaveBeenCalledWith('/projects/5/procfile')
    expect(res.data.processes).toHaveLength(2)
    expect(res.data.processes[0].proc_type).toBe('web')
  })
})
