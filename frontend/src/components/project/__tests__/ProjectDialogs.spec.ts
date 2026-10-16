import { describe, it, expect } from 'vitest'
import { nextTick } from 'vue'
import { mount } from '@vue/test-utils'
import ProjectFormDialog from '@/components/project/ProjectFormDialog.vue'
import WebhookDialog from '@/components/project/WebhookDialog.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'
import type { ProjectFormState } from '@/utils/projectFormat'

/**
 * T2-4 第六站拆出的两个弹窗契约（原为 ProjectView 内联 FpModal）。
 *
 * 两个弹窗共享「打开 → 编辑 → 上抛 → 成功由父级关」的骨架，但各有**不能丢的
 * 条件渲染**——它们是「用户看到什么决定了他能做什么」的直接来源：
 *
 * - 表单弹窗：编辑态 name 置灰 / git 才有仓库地址 / rolling 才有健康检查路径；
 * - 令牌弹窗：令牌与地址只读，且必须同屏提示「仅本次展示」。
 *
 * 弹窗经 Teleport 到 body，断言一律落在 `document.body`（T1-1 已记录的陷阱）。
 */

const mountLoose = mount as unknown as (
  c: unknown,
  o: Record<string, unknown>,
) => ReturnType<typeof mount>

const messages = {
  project: {
    createTitle: '新建项目',
    editTitle: '编辑项目',
    name: '项目名称',
    namePlaceholder: '字母、数字与 -/_，1–64 字符',
    kind: '项目类型',
    source: '来源',
    sourceUrl: 'Git 仓库地址',
    sourceUrlPlaceholder: 'https://...',
    runtime: '绑定运行时',
    runCommand: '启动命令',
    runCommandPlaceholder: '如 npm run start / java -jar app.jar',
    port: '端口',
    domain: '域名',
    deployStrategy: '切换策略',
    healthCheckPath: '健康检查路径',
    healthCheckPathPlaceholder: '如 /health（rolling 策略下零停机切换探测用）',
    webhookTitle: 'Webhook 部署令牌',
    webhookOnceHint: '部署令牌仅本次展示，请立即保存。重复开启将轮换令牌。',
    webhookToken: '部署令牌',
    webhookUrl: 'Webhook 地址',
    webhookPushHint: '将 Webhook 地址配置到 GitHub / GitLab / Gitee / Gitea 的 push 事件。',
    kinds: {
      php: 'PHP 站点',
      node: 'Node.js',
      static: '静态站点',
      jar: 'Java JAR',
      war: 'Java WAR',
      python: 'Python',
      go: 'Go',
    },
    sources: { local_path: '本地目录', git: 'Git 仓库' },
    strategies: { restart: '直接重启', rolling: '零停机切换（rolling）' },
  },
  common: { cancel: '取消', save: '保存', close: '关闭', copy: '复制', confirm: '确定' },
}

function mountDialog(component: unknown, props: Record<string, unknown> = {}) {
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

const row = (over: Record<string, unknown> = {}) => ({
  id: 3,
  name: 'blog',
  kind: 'node',
  source: 'git',
  source_url: 'https://example.com/blog.git',
  runtime_id: 7,
  run_command: 'npm start',
  port: 3000,
  domain: 'blog.example.com',
  health_check_path: '/health',
  deploy_strategy: 'rolling',
  status: 'running',
  ...over,
})

const runtimeOptions = [{ label: 'node 20', value: 7 }]

/** 弹窗 body 里的字段标签（FpSelect/FpInput 的 label 渲染成 <label>） */
function labels(): string[] {
  return Array.from(document.body.querySelectorAll('label')).map((l) => l.textContent ?? '')
}

describe('ProjectFormDialog · 打开与条件渲染', () => {
  it('初始不渲染（visible=false）', async () => {
    mountDialog(ProjectFormDialog, { runtimeOptions })
    await settle()
    expect(document.body.querySelector('.p-dialog')).toBeNull()
  })

  it('新建：标题「新建项目」，name 可编辑，默认静态站点 + 本地目录 + 直接重启', async () => {
    const w = mountDialog(ProjectFormDialog, { runtimeOptions })
    ;(w.vm as unknown as { open: (r?: unknown) => void }).open()
    await settle()

    expect(document.body.textContent).toContain('新建项目')
    const nameInput = document.body.querySelector('input') as HTMLInputElement
    expect(nameInput.disabled).toBe(false)
    expect(nameInput.value).toBe('')
    // 默认 static + local_path：没有仓库地址字段
    expect(labels()).not.toContain('Git 仓库地址')
    // 默认 restart：没有健康检查路径字段
    expect(labels()).not.toContain('健康检查路径')
  })

  it('【关键】编辑：标题「编辑项目」且 name 置灰（改名会让旧目录成为孤儿）', async () => {
    const w = mountDialog(ProjectFormDialog, { runtimeOptions })
    ;(w.vm as unknown as { open: (r: unknown) => void }).open(row())
    await settle()

    expect(document.body.textContent).toContain('编辑项目')
    const nameInput = document.body.querySelector('input') as HTMLInputElement
    expect(nameInput.disabled).toBe(true)
    expect(nameInput.value).toBe('blog')
  })

  it('【关键】git 来源 + rolling 策略：仓库地址与健康检查路径两个字段都出现', async () => {
    const w = mountDialog(ProjectFormDialog, { runtimeOptions })
    ;(w.vm as unknown as { open: (r: unknown) => void }).open(row())
    await settle()

    const all = labels()
    expect(all).toContain('Git 仓库地址')
    expect(all).toContain('健康检查路径')
  })

  it('【关键】本地目录来源：不显示仓库地址（没有 URL 可填）', async () => {
    const w = mountDialog(ProjectFormDialog, { runtimeOptions })
    ;(w.vm as unknown as { open: (r: unknown) => void }).open(row({ source: 'local_path' }))
    await settle()
    expect(labels()).not.toContain('Git 仓库地址')
  })

  it('非 rolling 策略：不显示健康检查路径（填了也不会被使用）', async () => {
    const w = mountDialog(ProjectFormDialog, { runtimeOptions })
    ;(w.vm as unknown as { open: (r: unknown) => void }).open(row({ deploy_strategy: 'restart' }))
    await settle()
    expect(labels()).not.toContain('健康检查路径')
  })

  it('运行时候选由 props 决定（数据层已按 kind 去重）', async () => {
    const w = mountDialog(ProjectFormDialog, { runtimeOptions: [{ label: 'php 8.2', value: 9 }] })
    ;(w.vm as unknown as { open: () => void }).open()
    await settle()
    expect(document.body.textContent).toContain('绑定运行时')
  })
})

describe('ProjectFormDialog · 上抛语义', () => {
  it('【关键】新建上抛 create（不带 id）；编辑上抛 update(id, form) 且不回传 name 之外的接口字段', async () => {
    const w = mountDialog(ProjectFormDialog, { runtimeOptions })

    ;(w.vm as unknown as { open: (r?: unknown) => void }).open()
    await settle()
    const createBtn = Array.from(document.body.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('保存'),
    )
    createBtn!.click()
    await settle()
    const created = (w.emitted() as Record<string, unknown[][]>).create
    expect(created).toBeTruthy()
    expect((created[0][0] as ProjectFormState).kind).toBe('static')

    ;(w.vm as unknown as { open: (r: unknown) => void }).open(row())
    await settle()
    const saveBtn = Array.from(document.body.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('保存'),
    )
    saveBtn!.click()
    await settle()
    const updated = (w.emitted() as Record<string, unknown[][]>).update
    expect(updated[0][0]).toBe(3)
    const form = updated[0][1] as ProjectFormState
    expect(form.name).toBe('blog')
    expect(form.source_url).toBe('https://example.com/blog.git')
    // 行数据里的请求态字段不进表单（表单只收「表单字段」）
    expect(form).not.toHaveProperty('status')
  })

  it('close()：隐藏弹窗（父级在请求成功后调用）', async () => {
    const w = mountDialog(ProjectFormDialog, { runtimeOptions })
    ;(w.vm as unknown as { open: () => void }).open()
    await settle()
    expect(document.body.querySelector('.p-dialog')).not.toBeNull()

    ;(w.vm as unknown as { close: () => void }).close()
    await settle()
    expect(document.body.querySelector('.p-dialog')).toBeNull()
  })

  it('重开新建会清掉上一次的编辑目标（不残留 id）', async () => {
    const w = mountDialog(ProjectFormDialog, { runtimeOptions })
    ;(w.vm as unknown as { open: (r: unknown) => void }).open(row({ id: 9 }))
    await settle()
    ;(w.vm as unknown as { open: (r?: unknown) => void }).open()
    await settle()

    const saveBtn = Array.from(document.body.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('保存'),
    )
    saveBtn!.click()
    await settle()
    const emitted = w.emitted() as Record<string, unknown[][]>
    expect(emitted.update).toBeFalsy()
    expect(emitted.create).toBeTruthy()
  })
})

describe('WebhookDialog', () => {
  it('打开即展示令牌、地址与「仅本次展示」提示（关掉就再也拿不到）', async () => {
    mountDialog(WebhookDialog, {
      modelValue: true,
      token: 'tok-123',
      url: '/api/projects/3/webhook?token=tok-123',
    })
    await settle()

    const text = document.body.textContent ?? ''
    expect(text).toContain('Webhook 部署令牌')
    expect(text).toContain('部署令牌仅本次展示，请立即保存。重复开启将轮换令牌。')
    expect(text).toContain('将 Webhook 地址配置到 GitHub / GitLab / Gitee / Gitea 的 push 事件。')

    const inputs = Array.from(document.body.querySelectorAll('input')) as HTMLInputElement[]
    expect(inputs.map((i) => i.value)).toContain('tok-123')
    expect(inputs.map((i) => i.value)).toContain('/api/projects/3/webhook?token=tok-123')
  })

  it('【关键】令牌与地址两字段只读（服务端事实，展示层无写回路径）', async () => {
    mountDialog(WebhookDialog, {
      modelValue: true,
      token: 'tok-123',
      url: '/api/projects/3/webhook?token=tok-123',
    })
    await settle()

    const inputs = Array.from(document.body.querySelectorAll('input')) as HTMLInputElement[]
    expect(inputs.every((i) => i.readOnly || i.disabled)).toBe(true)
  })

  it('点「复制」上抛 copy（拼接绝对地址与写剪贴板在数据层）', async () => {
    const w = mountDialog(WebhookDialog, { modelValue: true, token: 't', url: '/u' })
    await settle()

    const copyBtn = Array.from(document.body.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('复制'),
    )
    copyBtn!.click()
    await settle()
    expect((w.emitted() as Record<string, unknown[][]>).copy).toBeTruthy()
  })

  it('关闭按钮把可见性写回（v-model）', async () => {
    const w = mountDialog(WebhookDialog, { modelValue: true, token: 't', url: '/u' })
    await settle()

    const closeBtn = Array.from(document.body.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('关闭'),
    )
    closeBtn!.click()
    await settle()
    expect((w.emitted() as Record<string, unknown[][]>)['update:modelValue'][0][0]).toBe(false)
  })
})
