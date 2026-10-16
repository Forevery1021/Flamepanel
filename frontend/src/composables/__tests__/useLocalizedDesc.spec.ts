import { describe, it, expect } from 'vitest'
import { defineComponent, nextTick, type PropType } from 'vue'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { useLocalizedDesc } from '@/composables/useLocalizedDesc'

/**
 * T0-4 composable 级回归（Doc/23 §3.1 / 问题 P0-6）。
 *
 * 关键验收：描述取值必须**响应语言切换**——已渲染的卡片在切换语言后同步更新，
 * 而不是等组件重新挂载（历史实现绑死 `short_desc_zh`，切语言毫无变化）。
 */

const pkg = {
  name: 'Gitea',
  key: 'gitea',
  short_desc_zh: '轻量 Git 服务',
  short_desc_en: 'Lightweight Git hosting',
}

/**
 * 宿主组件统一经 `makeHost` 工厂构造（单处 defineComponent 调用），
 * 避免 spec 内堆叠组件定义触发 `vue/one-component-per-file`。
 * 描述对象经 prop 注入，同一组件覆盖「双字段 / 仅中文 / null」三类数据。
 */
function makeHost() {
  return defineComponent({
    props: {
      target: { type: Object as PropType<Record<string, unknown> | null>, default: null },
    },
    setup() {
      // props 在模板上下文自动可见，无需再返回（返回会与 props 同名冲突）
      const { localizedDesc, matchesQuery } = useLocalizedDesc()
      return { localizedDesc, matchesQuery }
    },
    template: '<div class="desc">{{ localizedDesc(target) }}</div>',
  })
}

/** 统一挂载入口：i18n 插件必须注入，否则 setup 内 useI18n 抛错 */
function mountHost(locale: string, target: Record<string, unknown> | null = pkg) {
  const i18n = createI18n({ legacy: false, locale, messages: {} })
  const w = mount(makeHost(), {
    props: { target },
    global: { plugins: [i18n] },
  })
  return { w, i18n }
}

describe('T0-4 useLocalizedDesc（响应式 + 取值链）', () => {
  it('zh 界面渲染中文描述', () => {
    const { w } = mountHost('zh-CN')
    expect(w.find('.desc').text()).toBe('轻量 Git 服务')
  })

  it('en 界面渲染英文描述', () => {
    const { w } = mountHost('en-US')
    expect(w.find('.desc').text()).toBe('Lightweight Git hosting')
  })

  it('ja 界面回落中文（后端无日语字段，缺失回退 zh）', () => {
    const { w } = mountHost('ja-JP')
    expect(w.find('.desc').text()).toBe('轻量 Git 服务')
  })

  it('【核心回归】切换语言后已渲染描述同步更新（无需重挂载）', async () => {
    const { w, i18n } = mountHost('zh-CN')
    expect(w.find('.desc').text()).toBe('轻量 Git 服务')

    ;(i18n.global.locale as unknown as { value: string }).value = 'en-US'
    await nextTick()
    expect(w.find('.desc').text()).toBe('Lightweight Git hosting')

    ;(i18n.global.locale as unknown as { value: string }).value = 'zh-CN'
    await nextTick()
    expect(w.find('.desc').text()).toBe('轻量 Git 服务')
  })

  it('en 界面英文字段缺失时回落中文', () => {
    const { w } = mountHost('en-US', { short_desc_zh: '仅中文简介' })
    expect(w.find('.desc').text()).toBe('仅中文简介')
  })

  it('null 包不抛错（列表加载中的空值）', () => {
    const { w } = mountHost('en-US', null)
    expect(w.find('.desc').text()).toBe('')
  })

  it('matchesQuery 中英关键词均可命中', () => {
    const { w } = mountHost('zh-CN')
    const vm = w.vm as unknown as { matchesQuery: (p: unknown, q: string) => boolean }
    expect(vm.matchesQuery(pkg, '轻量')).toBe(true)
    expect(vm.matchesQuery(pkg, 'lightweight')).toBe(true)
    expect(vm.matchesQuery(pkg, 'nope')).toBe(false)
    expect(vm.matchesQuery(null, 'x')).toBe(false)
  })
})
