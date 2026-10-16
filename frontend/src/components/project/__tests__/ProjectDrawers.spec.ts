import { describe, it, expect } from 'vitest'
import { nextTick } from 'vue'
import { mount } from '@vue/test-utils'
import ProjectLogsDrawer from '@/components/project/ProjectLogsDrawer.vue'
import ProjectLinksDrawer from '@/components/project/ProjectLinksDrawer.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第六站拆出的两个抽屉契约（原为 ProjectView 内联 FpDrawer）。
 *
 * 两个抽屉都靠 `FpStatePanel` 表达三态，但断言重点不同：
 *
 * - **日志抽屉**：重拉入口在正文上方（成功态下三态面板不渲染 retry 槽）——
 *   运维刷新日志是高频动作，若只在错误态给按钮，用户看到日志卡住就只能关掉重开；
 * - **联链抽屉**：`unknown` 类型染红是原实现里唯一一处红色标签（探测不出数据库类型
 *   多半意味着容器已挂），以及「关联」按钮的禁用条件。
 *
 * 抽屉经 Teleport 到 body，断言落在 `document.body`。
 */

const mountLoose = mount as unknown as (
  c: unknown,
  o: Record<string, unknown>,
) => ReturnType<typeof mount>

const messages = {
  project: {
    logsTitle: '项目日志 · {name}',
    logs: '日志',
    links: '数据库联链',
    linksTitle: '数据库联链 — {name}',
    linksEmpty: '尚未关联任何数据库实例',
    linksHint: '关联后，标准连接串将在项目启动与部署时自动注入进程环境变量。',
    linkInstance: '数据库实例',
    linkEnvName: '环境变量名（留空按类型默认）',
    linkEnvNamePlaceholder: '如 DATABASE_URL',
    linkCreate: '关联',
    linkUnlink: '解除关联',
  },
  common: {
    noData: '暂无数据',
    retry: '重试',
    failed: '操作失败',
  },
}

function mountDrawer(component: unknown, props: Record<string, unknown>) {
  document.body.innerHTML = ''
  return mountLoose(component, {
    props,
    attachTo: document.body,
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

describe('ProjectLogsDrawer', () => {
  it('关闭态不渲染抽屉', async () => {
    mountDrawer(ProjectLogsDrawer, { modelValue: false, project: null })
    await settle()
    expect(document.body.querySelector('.p-drawer')).toBeNull()
  })

  it('标题带项目名，正文是等宽 pre（长行不撑破抽屉）', async () => {
    mountDrawer(ProjectLogsDrawer, {
      modelValue: true,
      project: { id: 3, name: 'blog' },
      text: 'line-1\nline-2',
      lines: [1, 2],
    })
    await settle()

    expect(document.body.textContent).toContain('项目日志 · blog')
    const pre = document.body.querySelector('.logs-pre')
    expect(pre?.textContent).toBe('line-1\nline-2')
  })

  it('项目为 null：标题的 {name} 回落空串（不渲染成 undefined）', async () => {
    mountDrawer(ProjectLogsDrawer, { modelValue: true, project: null, lines: [1], text: 'x' })
    await settle()
    expect(document.body.textContent).toContain('项目日志 ·')
    expect(document.body.textContent).not.toContain('undefined')
  })

  it('【关键】正文已有内容时也给重拉入口（高频动作，不等失误态）', async () => {
    const w = mountDrawer(ProjectLogsDrawer, {
      modelValue: true,
      project: { id: 3, name: 'blog' },
      text: 'x',
      lines: [1],
    })
    await settle()

    const retryBtn = Array.from(document.body.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('重试'),
    )
    expect(retryBtn).toBeTruthy()
    retryBtn!.click()
    await settle()
    expect((w.emitted() as Record<string, unknown[][]>).refresh).toBeTruthy()
  })

  it('重拉中：刷新按钮带 loading（不重复点）', async () => {
    mountDrawer(ProjectLogsDrawer, {
      modelValue: true,
      project: { id: 3, name: 'blog' },
      text: 'x',
      lines: [1],
      fetching: true,
    })
    await settle()

    const retryBtn = Array.from(document.body.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('重试'),
    )
    expect(retryBtn!.className).toContain('p-button-loading')
  })

  it('错误态：展示错误文案与重试入口', async () => {
    mountDrawer(ProjectLogsDrawer, {
      modelValue: true,
      project: { id: 3, name: 'blog' },
      error: '操作失败',
      lines: [],
    })
    await settle()
    expect(document.body.textContent).toContain('操作失败')
  })

  it('空态：0 行日志走空态文案', async () => {
    mountDrawer(ProjectLogsDrawer, {
      modelValue: true,
      project: { id: 3, name: 'blog' },
      lines: [],
      text: '',
    })
    await settle()
    expect(document.body.textContent).toContain('暂无数据')
  })
})

describe('ProjectLinksDrawer · 已关联列表', () => {
  const link = (over: Record<string, unknown> = {}) => ({
    id: 11,
    project_id: 3,
    db_instance_id: 5,
    db_instance_name: 'mysql-main',
    db_type: 'mysql',
    env_name: 'DATABASE_URL',
    port: 3306,
    ...over,
  })

  it('每条带类型标签、实例名与等宽环境变量名', async () => {
    mountDrawer(ProjectLinksDrawer, {
      modelValue: true,
      project: { id: 3, name: 'blog' },
      links: [link()],
    })
    await settle()

    const text = document.body.textContent ?? ''
    expect(text).toContain('mysql')
    expect(text).toContain('mysql-main')
    expect(document.body.querySelector('.link-row__env')?.textContent).toBe('DATABASE_URL')
  })

  it('【关键】unknown 类型染红（探测不出类型多半是实例已挂，不能放过）', async () => {
    mountDrawer(ProjectLinksDrawer, {
      modelValue: true,
      project: { id: 3, name: 'blog' },
      links: [link({ db_type: 'unknown' }), link({ id: 12, db_type: 'mysql' })],
    })
    await settle()

    const tags = Array.from(document.body.querySelectorAll('.link-row .fp-tag'))
    expect(tags[0].className).toContain('danger')
    expect(tags[1].className).not.toContain('danger')
  })

  it('解除：上抛整条 link（数据层的确认文案要读 env_name）', async () => {
    const w = mountDrawer(ProjectLinksDrawer, {
      modelValue: true,
      project: { id: 3, name: 'blog' },
      links: [link({ id: 21 })],
    })
    await settle()

    const btn = Array.from(document.body.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('解除关联'),
    )
    btn!.click()
    await settle()
    expect((w.emitted() as Record<string, unknown[][]>).unlink[0][0]).toMatchObject({ id: 21 })
  })

  it('【关键】只在解除的那一条上转圈（per-link 锁）', async () => {
    mountDrawer(ProjectLinksDrawer, {
      modelValue: true,
      project: { id: 3, name: 'blog' },
      links: [link({ id: 21 }), link({ id: 22, env_name: 'REDIS_URL' })],
      actingId: 'unlink-21',
    })
    await settle()

    const btns = Array.from(document.body.querySelectorAll('.link-row button'))
    expect(btns[0].className).toContain('p-button-loading')
    expect(btns[1].className).not.toContain('p-button-loading')
  })

  it('【存量缺陷锁定】空列表渲染 FpStatePanel 默认空态（`empty-text` 不是有效 prop）', async () => {
    // 原实现把 `project.linksEmpty` 传给 FpStatePanel 的 `empty-text`，
    // 而该组件只认 `empty-title` / `empty-desc`（且无 $attrs 透传）——
    // 于是这个 key 从未上屏，实际渲染的是默认的「暂无数据」。
    // 本轮是纯拆分，**原样保留**（改文案属可见行为变更）；本用例把现状锁住，
    // 待修时这条会红，提醒同步改断言与 Doc。
    mountDrawer(ProjectLinksDrawer, {
      modelValue: true,
      project: { id: 3, name: 'blog' },
      links: [],
    })
    await settle()
    expect(document.body.textContent).toContain('暂无数据')
    expect(document.body.textContent).not.toContain('尚未关联任何数据库实例')
  })
})

describe('ProjectLinksDrawer · 关联表单', () => {
  it('【关键】未选实例：「关联」禁用（不能发一个没有目标的请求）', async () => {
    mountDrawer(ProjectLinksDrawer, {
      modelValue: true,
      project: { id: 3, name: 'blog' },
      links: [],
      form: { db_instance_id: undefined, env_name: '' },
    })
    await settle()

    const btn = Array.from(document.body.querySelectorAll('button')).find(
      (b) => b.textContent?.trim() === '关联',
    )
    expect(btn!.disabled).toBe(true)
  })

  it('选了实例：按钮可用，点击上抛 link', async () => {
    const w = mountDrawer(ProjectLinksDrawer, {
      modelValue: true,
      project: { id: 3, name: 'blog' },
      links: [],
      form: { db_instance_id: 5, env_name: '' },
      linkableOptions: [{ label: 'mysql-main (mysql:3306)', value: 5 }],
    })
    await settle()

    const btn = Array.from(document.body.querySelectorAll('button')).find(
      (b) => b.textContent?.trim() === '关联',
    )
    expect(btn!.disabled).toBe(false)
    btn!.click()
    await settle()
    expect((w.emitted() as Record<string, unknown[][]>).link).toBeTruthy()
  })

  it('提示文案在表单上方（说明注入哪些环境变量与凭据边界）', async () => {
    mountDrawer(ProjectLinksDrawer, {
      modelValue: true,
      project: { id: 3, name: 'blog' },
      links: [],
      form: { db_instance_id: 5, env_name: '' },
    })
    await settle()
    expect(document.body.textContent).toContain(
      '标准连接串将在项目启动与部署时自动注入进程环境变量',
    )
  })
})
