import { describe, it, expect } from 'vitest'
import { queryKeys } from '@/api/queryKeys'

describe('queryKeys（F1.1 统一 queryKey 约定）', () => {
  it('节点列表 key 稳定且包含分页参数', () => {
    expect(queryKeys.nodes.list(1, 20)).toEqual(['nodes', 'list', 1, 20])
    expect(queryKeys.nodes.list(2, 50)).toEqual(['nodes', 'list', 2, 50])
  })

  it('同一 key 同屏共享缓存（相同参数 → 相同 key）', () => {
    expect(queryKeys.nodes.list(1, 20)).toEqual(queryKeys.nodes.list(1, 20))
    expect(queryKeys.nodes.list(1, 20)).not.toEqual(queryKeys.nodes.list(1, 50))
  })

  it('全量前缀 key 可用于 invalidate 一类查询', () => {
    // invalidate ['nodes'] 会失效 nodes.list(...) 等派生 key（前缀匹配）
    expect(queryKeys.nodes.all[0]).toBe('nodes')
    expect(queryKeys.nodes.list()[0]).toBe('nodes')
  })

  it('文件列表按路径区分缓存', () => {
    expect(queryKeys.files.list('/etc')).toEqual(['files', 'list', '/etc'])
    expect(queryKeys.files.list('/etc/nginx')).toEqual(['files', 'list', '/etc/nginx'])
  })
})
