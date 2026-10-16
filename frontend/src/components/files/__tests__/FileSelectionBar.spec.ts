import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import FileSelectionBar from '@/components/files/FileSelectionBar.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

const messages = {
  file: {
    selected: '已选 {n} 项',
    selectionActions: '批量操作',
    compress: '压缩',
    copyTo: '复制到',
    moveTo: '移动到',
    delete: '删除',
    clearSelection: '取消选择',
  },
}

describe('FileSelectionBar', () => {
  it('显示计数（polite 播报）并把每个按钮映射到事件', async () => {
    const w = mount(FileSelectionBar, {
      props: { count: 3 },
      global: {
        plugins: [createTestI18n(messages), openVuePlugin, ...services],
        directives: { permission: () => {} },
      },
    })
    expect(w.attributes('role')).toBe('region')
    expect(w.attributes('aria-label')).toBe('批量操作')
    expect(w.find('[aria-live="polite"]').text()).toBe('已选 3 项')
    const map: Record<string, string> = {
      压缩: 'compress',
      复制到: 'copy',
      移动到: 'move',
      删除: 'delete',
      取消选择: 'clear',
    }
    for (const [label, event] of Object.entries(map)) {
      await w
        .findAll('button')
        .find((b) => b.text() === label)!
        .trigger('click')
      expect(w.emitted(event)).toHaveLength(1)
    }
  })
})
