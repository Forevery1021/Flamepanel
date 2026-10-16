import { describe, it, expect } from 'vitest'
import { nextTick } from 'vue'
import { mount } from '@vue/test-utils'
import ContainerLogsDialog from '@/components/docker/ContainerLogsDialog.vue'
import ContainerJsonDialog from '@/components/docker/ContainerJsonDialog.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第三站拆出的两个只读预览弹窗契约（原为 DockerView 内联 FpModal）。
 *
 * 二者都是「标题 + 一段文本」，但空态语义不同——日志空态是 `docker.noLogs`，
 * 统计空态是 `docker.noStats`，详情空态回落 `common.noData`。这些占位文案是
 * 用户判断「真没有数据」还是「没加载出来」的唯一线索，故逐个锁死。
 */

const mountLoose = mount as unknown as (
  c: unknown,
  o: Record<string, unknown>,
) => ReturnType<typeof mount>

const messages = {
  docker: {
    containerLogs: '容器日志',
    containerStats: '容器统计',
    detail: '详情',
    refresh: '刷新',
    tailLines: '显示行数',
    noLogs: '暂无日志',
    noStats: '暂无统计数据',
  },
  common: { noData: '暂无数据' },
}

function mountDialog(component: unknown, props: Record<string, unknown>) {
  document.body.innerHTML = ''
  return mountLoose(component, {
    props,
    global: {
      plugins: [createTestI18n(messages), openVuePlugin, ...services],
      directives: { permission: () => {} },
    },
  })
}

async function settle() {
  await nextTick()
  await nextTick()
  await nextTick()
}

describe('ContainerLogsDialog', () => {
  it('标题拼接容器展示名（名称回退 ID 由父级算好）', async () => {
    mountDialog(ContainerLogsDialog, { visible: true, container: 'web', content: 'hello' })
    await settle()
    expect(document.body.textContent).toContain('容器日志 — web')
  })

  it('内容原样渲染在终端样式的 pre 里（不转义、不改写）', async () => {
    mountDialog(ContainerLogsDialog, {
      visible: true,
      container: 'web',
      content: '<script>alert(1)</script>\nline2',
    })
    await settle()

    const pre = document.body.querySelector('.log-viewer') as HTMLElement
    expect(pre.textContent).toBe('<script>alert(1)</script>\nline2')
    // 尖括号作为文本节点存在（而不是被解析成元素）
    expect(pre.querySelector('script')).toBeNull()
  })

  it('内容为空：渲染「暂无日志」占位（不是空白 pre）', async () => {
    mountDialog(ContainerLogsDialog, { visible: true, container: 'web', content: '' })
    await settle()
    expect(document.body.querySelector('.log-viewer')?.textContent).toBe('暂无日志')
  })

  it('行数候选四档（三档 + 「全部」哨兵 9999），切换经 v-model:tail 上抛', async () => {
    const w = mountDialog(ContainerLogsDialog, { visible: true, container: 'web', tail: 100 })
    await settle()

    // FpSelect 未 `defineProps` options（经 `v-bind="$attrs"` 直通底层 Select），
    // 故从组件实例的 `$attrs` 读取——这正是真实渲染时传给底层的那一份。
    const select = w.findAllComponents({ name: 'FpSelect' })[0]
    const options = (
      select.vm as unknown as { $attrs: { options: { label: string; value: number }[] } }
    ).$attrs.options
    expect(options.map((o) => o.value)).toEqual([50, 100, 500, 9999])

    select.vm.$emit('update:modelValue', 500)
    await settle()
    expect(w.emitted('update:tail')?.[0]).toEqual([500])
  })

  it('刷新按钮上抛 refresh（行数变更后由用户显式触发重取）', async () => {
    const w = mountDialog(ContainerLogsDialog, { visible: true, container: 'web' })
    await settle()

    const btn = [...document.body.querySelectorAll('button')].find((b) =>
      b.textContent?.includes('刷新'),
    ) as HTMLButtonElement
    btn.click()
    await settle()
    expect(w.emitted('refresh')).toBeTruthy()
  })
})

describe('ContainerJsonDialog（统计 / 详情共用）', () => {
  it('plain 变体：JSON 按原文渲染在等宽文本节点里', async () => {
    mountDialog(ContainerJsonDialog, {
      visible: true,
      headerKey: 'docker.containerStats',
      container: 'web',
      json: '{"cpu":"1.5%"}',
    })
    await settle()

    expect(document.body.textContent).toContain('容器统计 — web')
    expect(document.body.querySelector('.json-viewer')).toBeNull()
    expect(document.body.textContent).toContain('{"cpu":"1.5%"}')
  })

  it('terminal 变体：用暗色终端底样式（详情弹窗与官方文档对照时更易读层级）', async () => {
    mountDialog(ContainerJsonDialog, {
      visible: true,
      headerKey: 'docker.detail',
      container: 'web',
      json: '{"Id":"c1"}',
      variant: 'terminal',
    })
    await settle()
    expect(document.body.querySelector('.json-viewer')?.textContent).toBe('{"Id":"c1"}')
  })

  it('空 JSON：渲染调用方传入的空态文案（两个弹窗文案不同，故参数化）', async () => {
    mountDialog(ContainerJsonDialog, {
      visible: true,
      headerKey: 'docker.containerStats',
      container: 'web',
      json: '',
      emptyText: '暂无统计数据',
    })
    await settle()
    expect(document.body.textContent).toContain('暂无统计数据')
  })

  it('宽度参数化（统计 700 / 详情 900，窄屏下不统一放大）', async () => {
    mountDialog(ContainerJsonDialog, {
      visible: true,
      headerKey: 'docker.detail',
      container: 'web',
      json: '{}',
      width: '900px',
    })
    await settle()
    const dialog = document.body.querySelector('.p-dialog') as HTMLElement
    expect(dialog.style.width).toBe('900px')
  })
})
