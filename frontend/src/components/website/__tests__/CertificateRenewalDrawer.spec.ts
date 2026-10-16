import { describe, it, expect } from 'vitest'
import { nextTick } from 'vue'
import { mount } from '@vue/test-utils'
import CertificateRenewalDrawer from '@/components/website/CertificateRenewalDrawer.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * S-10 证书续签记录抽屉契约。
 *
 * - 概况区：开关三态（已开启 / 已关闭 / 不支持）由 `supports_auto_renew` + `auto_renew` 决定；
 * - 时间线：四类事件各自的文案与附加信息（失败原因 / 到期时间 / 来源），顺序按后端给的原样；
 * - 截断提示与空态；重拉按钮成功态也在（与日志抽屉同构）。
 *
 * 抽屉经 Teleport 到 body，断言落在 `document.body`。
 */

const mountLoose = mount as unknown as (
  c: unknown,
  o: Record<string, unknown>,
) => ReturnType<typeof mount>

const messages = {
  certificate: {
    autoRenew: '自动续签',
    expiresAt: '到期时间',
    lastRenewedAt: '最近签发',
    neverRenewed: '—',
    source_letsencrypt: "Let's Encrypt",
    renewalHistoryTitle: '续签记录 · {domain}',
    autoRenewOn: '已开启',
    autoRenewOff: '已关闭',
    autoRenewNa: '不支持',
    renewalWindow: '续签窗口',
    renewalWindowFrom: '{time} 起',
    scanInterval: '扫描周期',
    scanIntervalValue: '每 {n} 小时',
    timelineEmpty: '暂无续签记录',
    timelineTruncated: '仅显示最近 {n} 条',
    event_issued: '签发',
    event_auto_renewed: '自动续签成功',
    event_auto_renew_failed: '自动续签失败',
    event_expiry_warning: '到期预警',
    eventExpiresAt: '到期时间 {time}',
    eventSource: '来源：{source}',
  },
  common: { retry: '重试', noData: '暂无数据', failed: '操作失败' },
}

const cert = {
  id: 7,
  domains: '["a.example.com","b.example.com"]',
  source: 'letsencrypt',
  auto_renew: true,
}

const history = (over: Record<string, unknown> = {}) => ({
  certificate_id: 7,
  auto_renew: true,
  supports_auto_renew: true,
  expires_at: '2026-12-01T00:00:00Z',
  renewal_window_starts_at: '2026-11-01T00:00:00Z',
  last_renewed_at: '2026-09-02T00:00:00Z',
  scan_interval_hours: 6,
  truncated: false,
  entries: [
    { event_id: 4, kind: 'auto_renewed', domain: 'a.example.com', at: '2026-09-02T00:00:00Z' },
    {
      event_id: 3,
      kind: 'auto_renew_failed',
      domain: 'a.example.com',
      error: 'ACME 目录不可达',
      at: '2026-09-01T18:00:00Z',
    },
    {
      event_id: 2,
      kind: 'expiry_warning',
      domain: 'a.example.com',
      expires_at: '2026-09-20T00:00:00Z',
      at: '2026-09-01T12:00:00Z',
    },
    {
      event_id: 1,
      kind: 'issued',
      domain: 'a.example.com',
      source: 'letsencrypt',
      at: '2026-06-20T00:00:00Z',
    },
  ],
  ...over,
})

function mountDrawer(props: Record<string, unknown>) {
  document.body.innerHTML = ''
  return mountLoose(CertificateRenewalDrawer, {
    props: { modelValue: true, certificate: cert, ...props },
    attachTo: document.body,
    global: { plugins: [createTestI18n(messages), openVuePlugin, ...services] },
  })
}

async function settle() {
  await nextTick()
  await nextTick()
  await nextTick()
}

const text = () => document.body.textContent ?? ''

describe('CertificateRenewalDrawer', () => {
  it('关闭态不渲染抽屉', async () => {
    mountDrawer({ modelValue: false, history: history() })
    await settle()
    expect(document.body.querySelector('.p-drawer')).toBeNull()
  })

  it('标题带域名；概况区展示开关态、续签窗口与扫描周期', async () => {
    mountDrawer({ history: history() })
    await settle()
    expect(text()).toContain('续签记录 · a.example.com, b.example.com')
    expect(text()).toContain('已开启')
    expect(text()).toContain(`${new Date('2026-11-01T00:00:00Z').toLocaleString()} 起`)
    expect(text()).toContain('每 6 小时')
  })

  it('开关三态：不支持 > 已关闭 > 已开启', async () => {
    mountDrawer({ history: history({ supports_auto_renew: false, auto_renew: true }) })
    await settle()
    expect(text()).toContain('不支持')
    mountDrawer({ history: history({ auto_renew: false }) })
    await settle()
    expect(text()).toContain('已关闭')
  })

  it('时间线按原序渲染四类事件，附带失败原因 / 到期时间 / 来源', async () => {
    mountDrawer({ history: history() })
    await settle()
    const items = [...document.body.querySelectorAll('.renewal-timeline__item')]
    expect(items.map((li) => li.querySelector('.renewal-timeline__head')?.textContent)).toEqual(
      expect.arrayContaining([expect.stringContaining('自动续签成功')]),
    )
    expect(items).toHaveLength(4)
    expect(items[0]!.textContent).toContain('自动续签成功')
    expect(items[1]!.textContent).toContain('ACME 目录不可达')
    expect(items[1]!.classList).toContain('renewal-timeline__item--danger')
    expect(items[2]!.textContent).toContain(
      `到期时间 ${new Date('2026-09-20T00:00:00Z').toLocaleString()}`,
    )
    expect(items[3]!.textContent).toContain("来源：Let's Encrypt")
  })

  it('空时间线显示空态文案；截断时提示条数', async () => {
    mountDrawer({ history: history({ entries: [] }) })
    await settle()
    expect(text()).toContain('暂无续签记录')

    mountDrawer({ history: history({ truncated: true }) })
    await settle()
    expect(text()).toContain('仅显示最近 4 条')
  })

  it('成功态也有重拉按钮，点击上抛 refresh', async () => {
    const w = mountDrawer({ history: history() })
    await settle()
    const btn = [...document.body.querySelectorAll('button')].find((b) =>
      b.textContent?.includes('重试'),
    )
    expect(btn).toBeTruthy()
    btn!.click()
    await settle()
    expect(w.emitted('refresh')).toBeTruthy()
  })
})
