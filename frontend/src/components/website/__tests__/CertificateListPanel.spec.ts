import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { h, type VNode } from 'vue'
import CertificateListPanel from '@/components/website/CertificateListPanel.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第二站拆出的证书列表契约（原为 WebsitesView 内联表格）。
 *
 * 证书页签的到期列有三种形态（已过期 / 30 天内到期 / 正常时间），全部由纯函数
 * `certState` 判定；本 spec 在**组件层**再验一次映射（纯函数单测锁边界，组件层
 * 锁「边界 → 具体文案与 severity」这一跳）。
 *
 * 表格替身取舍同 WebsiteListPanel.spec（jsdom 无排版引擎，虚拟滚动不可断言）。
 */

type ColumnVNode = VNode & {
  props: (Record<string, unknown> & { field?: string; header?: string }) | null
  children: Record<string, (scope: { data: Record<string, unknown> }) => unknown> | null
}

/** 表格替身（纯对象，理由见 WebsiteListPanel.spec 顶部注释） */
const TableStub = {
  name: 'FpTable',
  props: {
    rows: { type: Array, default: () => [] },
    loading: { type: Boolean, default: false },
    emptyText: { type: String, default: '' },
  },
  setup(
    props: { rows: Record<string, unknown>[]; emptyText: string },
    { slots }: { slots: { default?: () => unknown } },
  ) {
    return () => {
      const columns = (slots.default?.() ?? []) as ColumnVNode[]
      if (!props.rows.length) return h('div', { class: 'stub-empty' }, props.emptyText)
      return h(
        'div',
        { class: 'stub-table' },
        props.rows.map((row, index) =>
          h(
            'div',
            { class: 'stub-row', 'data-p-index': index },
            columns.map((col, colIndex) => {
              const field = col.props?.field as string | undefined
              const body = col.children?.body
              return h(
                'div',
                {
                  class: 'stub-cell',
                  'data-col': colIndex,
                  'data-header': String(col.props?.header ?? ''),
                },
                body ? (body({ data: row }) as never) : String(field ? (row[field] ?? '') : ''),
              )
            }),
          ),
        ),
      )
    }
  },
}

const messages = {
  website: { id: 'ID' },
  certificate: {
    domains: '域名列表',
    source: '来源',
    source_letsencrypt: "Let's Encrypt",
    source_zerossl: 'ZeroSSL',
    source_self_signed: '自签',
    source_uploaded: '上传',
    expiresAt: '到期时间',
    expired: '已过期',
    expiringSoon: '即将到期',
    neverRenewed: '—',
    autoRenew: '自动续签',
    boundWebsite: '关联网站',
    notBound: '未绑定',
    bind: '绑定网站',
    unbind: '解绑',
    renewNow: '续签',
    renewalHistory: '续签记录',
    autoRenewUnsupported: '仅 ACME 签发的证书支持自动续签',
  },
  common: { operation: '操作', delete: '删除', noData: '暂无数据' },
}

const future = (days: number) => new Date(Date.now() + days * 86400000).toISOString()

const cert = (over: Record<string, unknown> = {}) => ({
  id: 1,
  domains: '["example.com","*.example.com"]',
  source: 'letsencrypt',
  expires_at: future(200),
  auto_renew: true,
  website_id: null,
  created_at: '',
  updated_at: '',
  ...over,
})

/** 状态面板替身：保留三态语义（空/错误/内容），便于独立断言 */
const StatePanelStub = {
  name: 'FpStatePanel',
  props: {
    loading: { type: Boolean, default: false },
    error: { type: String, default: null },
    empty: { type: Boolean, default: false },
  },
  emits: ['retry'],
  setup(
    props: { loading: boolean; error: string | null; empty: boolean },
    { slots }: { slots: { default?: () => unknown } },
  ) {
    return () =>
      h(
        'div',
        {
          class: 'stub-state',
          'data-loading': String(props.loading),
          'data-error': props.error ?? '',
          'data-empty': String(props.empty),
        },
        props.error
          ? h('div', { class: 'stub-state-error' }, props.error)
          : props.empty
            ? h('div', { class: 'stub-state-empty' }, '空')
            : (slots.default?.() as never),
      )
  },
}

const mountLoose = mount as unknown as (
  c: unknown,
  o: Record<string, unknown>,
) => ReturnType<typeof mount>

function mountPanel(props: Record<string, unknown> = {}) {
  return mountLoose(CertificateListPanel, {
    props: { certificates: [cert()], ...props },
    global: {
      plugins: [createTestI18n(messages), openVuePlugin, ...services],
      directives: { permission: () => {}, tooltip: () => {} },
      stubs: { FpTable: TableStub, FpStatePanel: StatePanelStub },
    },
  })
}

function actionLabels(w: ReturnType<typeof mountPanel>): string[] {
  return w.findAll('.row-actions button').map((b) => b.text())
}

describe('CertificateListPanel · 列渲染', () => {
  it('声明七列（ID/域名/来源/到期时间/自动续签/关联网站/操作）', () => {
    const w = mountPanel()
    const headers = w.findAll('.stub-row .stub-cell').map((c) => c.attributes('data-header'))
    expect(headers).toEqual(['ID', '域名列表', '来源', '到期时间', '自动续签', '关联网站', '操作'])
  })

  it('域名列：SAN JSON 解析为逗号分隔（含泛域名）', () => {
    const w = mountPanel()
    expect(w.find('[data-col="1"]').text()).toBe('example.com, *.example.com')
  })

  it('来源列：letsencrypt / self_signed / uploaded 各自的译文', () => {
    expect(mountPanel({ certificates: [cert({ source: 'letsencrypt' })] }).text()).toContain(
      "Let's Encrypt",
    )
    expect(mountPanel({ certificates: [cert({ source: 'self_signed' })] }).text()).toContain('自签')
    expect(mountPanel({ certificates: [cert({ source: 'uploaded' })] }).text()).toContain('上传')
  })

  it('到期列：正常时间显示本地化时间串', () => {
    const iso = future(200)
    const w = mountPanel({ certificates: [cert({ expires_at: iso })] })
    expect(w.find('[data-col="3"]').text()).toBe(new Date(iso).toLocaleString())
  })

  it('到期列：已过期 → 「已过期」标签（不是时间串）', () => {
    const w = mountPanel({ certificates: [cert({ expires_at: future(-1) })] })
    expect(w.find('[data-col="3"]').text()).toBe('已过期')
  })

  it('到期列：30 天内 → 「即将到期」标签', () => {
    const w = mountPanel({ certificates: [cert({ expires_at: future(10) })] })
    expect(w.find('[data-col="3"]').text()).toBe('即将到期')
  })

  it('到期列：无到期时间 → 占位文案（未签发/待上传）', () => {
    const w = mountPanel({ certificates: [cert({ expires_at: null })] })
    expect(w.find('[data-col="3"]').text()).toBe('—')
  })

  it('关联网站：有值时显示 id，未绑定显示占位文案', () => {
    expect(mountPanel({ certificates: [cert({ website_id: 7 })] }).text()).toContain('7')
    expect(mountPanel({ certificates: [cert({ website_id: null })] }).text()).toContain('未绑定')
  })
})

describe('CertificateListPanel · 操作列', () => {
  it('未绑定 + 自签来源：绑定 + 删除（无解绑、无续签）', () => {
    expect(
      actionLabels(
        mountPanel({ certificates: [cert({ website_id: null, source: 'self_signed' })] }),
      ),
    ).toEqual(['绑定网站', '续签记录', '删除'])
  })

  it('已绑定：绑定 + 解绑 + 删除', () => {
    expect(
      actionLabels(mountPanel({ certificates: [cert({ website_id: 3, source: 'self_signed' })] })),
    ).toEqual(['绑定网站', '解绑', '续签记录', '删除'])
  })

  it('ACME 来源（letsencrypt/zerossl）：多一个「续签」按钮', () => {
    expect(actionLabels(mountPanel({ certificates: [cert({ source: 'zerossl' })] }))).toEqual([
      '绑定网站',
      '续签记录',
      '续签',
      '删除',
    ])
  })

  it('【关键】自签/上传来源无续签按钮（后端对这两类返回 400，前端不得给出入口）', () => {
    for (const source of ['self_signed', 'uploaded']) {
      const labels = actionLabels(mountPanel({ certificates: [cert({ source })] }))
      expect(labels).not.toContain('续签')
    }
  })

  it('续签按钮 loading 只锁当前行（renewingId 命中的行进入 loading 态）', () => {
    const busy = mountPanel({ certificates: [cert({ id: 9 })], renewingId: 9 })
    const idle = mountPanel({ certificates: [cert({ id: 9 })], renewingId: 8 })
    const pick = (w: ReturnType<typeof mountPanel>) =>
      w.findAll('.row-actions button').find((b) => b.text() === '续签')!

    // FpButton 的 loading 落在 class 与 data-p-disabled 上（非原生 disabled 属性）
    expect(pick(busy).html()).not.toBe(pick(idle).html())
    expect(pick(busy).classes()).toContain('p-button-loading')
    expect(pick(idle).classes()).not.toContain('p-button-loading')
  })

  it('四类操作各带上整行数据上抛', async () => {
    const w = mountPanel({ certificates: [cert({ id: 55, website_id: 3 })] })
    for (const b of w.findAll('.row-actions button')) await b.trigger('click')

    for (const ev of ['bind', 'unbind', 'history', 'renew', 'delete']) {
      expect((w.emitted(ev)?.[0]?.[0] as { id: number }).id).toBe(55)
    }
  })
})

describe('CertificateListPanel · auto_renew 开关', () => {
  it('开关按行值渲染并上抛 update 事件（不改 props，回写在数据层）', async () => {
    const w = mountPanel({ certificates: [cert({ id: 3, auto_renew: false })] })
    const toggle = w.findComponent({ name: 'FpSwitch' })
    expect(toggle.exists()).toBe(true)

    toggle.vm.$emit('update:modelValue', true)
    await w.vm.$nextTick()

    const payload = w.emitted('toggleAutoRenew')?.[0]
    expect((payload?.[0] as { id: number }).id).toBe(3)
    expect(payload?.[1]).toBe(true)
  })
})

describe('CertificateListPanel · S-10 非 ACME 证书开关禁用', () => {
  it('自签/上传：开关恒为关闭 + 禁用，并带原因提示（即便老库 auto_renew 为真）', async () => {
    for (const source of ['self_signed', 'uploaded']) {
      const w = mountPanel({ certificates: [cert({ source, auto_renew: true })] })
      const toggle = w.findComponent({ name: 'FpSwitch' })
      expect(toggle.props('disabled')).toBe(true)
      expect(toggle.props('modelValue')).toBe(false)
      expect(w.find('.auto-renew-na').exists()).toBe(true)
      toggle.vm.$emit('update:modelValue', true)
      await w.vm.$nextTick()
      expect(w.emitted('toggleAutoRenew')).toBeFalsy()
    }
  })

  it('ACME 证书：开关可用，无禁用包裹', () => {
    const w = mountPanel({ certificates: [cert({ source: 'letsencrypt' })] })
    expect(w.findComponent({ name: 'FpSwitch' }).props('disabled')).toBe(false)
    expect(w.find('.auto-renew-na').exists()).toBe(false)
  })
})

describe('CertificateListPanel · 空态与重试', () => {
  it('空列表 → 父级空态判定成立（非 loading 非 error 且无数据）', () => {
    const w = mountPanel({ certificates: [] })
    expect(w.find('.stub-state').attributes('data-empty')).toBe('true')
  })

  it('错误态优先级高于空态（有 error 时不显示「暂无数据」，避免误导）', () => {
    const w = mountPanel({ certificates: [], error: '加载失败' })
    const state = w.find('.stub-state')
    expect(state.attributes('data-error')).toBe('加载失败')
    expect(w.find('.stub-state-empty').exists()).toBe(false)
  })

  it('retry 事件上抛（状态面板的重试按钮接父级刷新）', async () => {
    const w = mountPanel({ error: 'boom' })
    w.findComponent(StatePanelStub).vm.$emit('retry')
    await w.vm.$nextTick()
    expect(w.emitted('retry')).toBeTruthy()
  })
})
