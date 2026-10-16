import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import RunImportDialog from '@/components/docker/RunImportDialog.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/** `docker run` 粘贴导入弹窗：只收集命令并上抛，请求与关闭由父级编排。 */

const messages = {
  common: { cancel: '取消', required: '此项为必填项' },
  docker: {
    importRun: '从 docker run 导入',
    importRunHint: '提示',
    runCommand: 'docker run 命令',
    runCommandPlaceholder: 'docker run …',
    convert: '转换',
  },
}

function mountDialog() {
  return mount(RunImportDialog, {
    attachTo: document.body,
    global: {
      plugins: [createTestI18n(messages), openVuePlugin, ...services],
      stubs: {
        FpModal: {
          props: ['modelValue'],
          template:
            '<div v-if="modelValue" class="stub-modal"><slot /><slot name="footer" /></div>',
        },
      },
    },
  })
}

type Exposed = { open: () => void; close: () => void }

describe('RunImportDialog', () => {
  it('open() 后显示；空命令提交 → 必填错误且不上抛', async () => {
    const w = mountDialog()
    ;(w.vm as unknown as Exposed).open()
    await w.vm.$nextTick()
    expect(w.find('.stub-modal').exists()).toBe(true)
    await w.find('form').trigger('submit')
    expect(w.emitted('submit')).toBeUndefined()
    expect(w.text()).toContain('此项为必填项')
    w.unmount()
  })

  it('提交去除首尾空白后的命令；弹窗保持打开（由父级在成功后关闭）', async () => {
    const w = mountDialog()
    ;(w.vm as unknown as Exposed).open()
    await w.vm.$nextTick()
    await w.find('textarea').setValue('  docker run -d nginx:alpine \n')
    await w.find('form').trigger('submit')
    expect(w.emitted('submit')?.[0]).toEqual(['docker run -d nginx:alpine'])
    expect(w.find('.stub-modal').exists()).toBe(true)
    ;(w.vm as unknown as Exposed).close()
    await w.vm.$nextTick()
    expect(w.find('.stub-modal').exists()).toBe(false)
    w.unmount()
  })
})
