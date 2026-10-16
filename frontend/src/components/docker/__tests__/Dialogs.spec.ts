import { describe, it, expect } from 'vitest'
import { nextTick } from 'vue'
import { mount } from '@vue/test-utils'
import RenameDialog from '@/components/docker/RenameDialog.vue'
import CreateNetworkDialog from '@/components/docker/CreateNetworkDialog.vue'
import CreateVolumeDialog from '@/components/docker/CreateVolumeDialog.vue'
import ConnectContainerDialog from '@/components/docker/ConnectContainerDialog.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第三站拆出的四个表单弹窗契约（原为 DockerView 内联 FpModal）。
 *
 * 四个弹窗共享同一套「打开 → 校验 → 上抛 → 成功由父级关」的骨架，但各有
 * **一条不能丢的本地校验**（空名称 / 空容器）。这些校验在拆分前是内联
 * `if (!x.trim()) { error = ...; return }`，最容易被「数据层也会校验」说服着删掉——
 * 但两者的用户可见效果完全不同：本地校验给你字段旁的红字，数据层的空值检查
 * 只会静默返回（用户以为点了没反应）。
 *
 * 弹窗经 Teleport 到 body，断言一律落在 `document.body`（T1-1 已记录的陷阱）。
 */

const mountLoose = mount as unknown as (
  c: unknown,
  o: Record<string, unknown>,
) => ReturnType<typeof mount>

const messages = {
  docker: {
    rename: '重命名',
    name: '名称',
    renamePlaceholder: '新容器名称',
    createNetwork: '创建网络',
    driver: '驱动',
    subnet: '子网',
    createVolume: '创建卷',
    connectContainer: '连接容器',
    container: '容器',
  },
  common: {
    cancel: '取消',
    confirm: '确定',
    required: '此项为必填项',
  },
}

function mountDialog(component: unknown, props: Record<string, unknown> = {}) {
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

const container = { id: 'c1', name: 'web', status: 'running' }

describe('RenameDialog', () => {
  it('初始不渲染（visible=false）', async () => {
    mountDialog(RenameDialog)
    await settle()
    expect(document.body.querySelector('.p-dialog')).toBeNull()
  })

  it('open(row)：以当前容器名为初值打开，标题与输入框就位', async () => {
    const w = mountDialog(RenameDialog)
    ;(w.vm as unknown as { open: (r: unknown) => void }).open(container)
    await settle()

    expect(document.body.textContent).toContain('重命名')
    const input = document.body.querySelector('input') as HTMLInputElement
    expect(input.value).toBe('web')
  })

  it('名称为空：不关闭、不上抛，字段旁出现必填错误', async () => {
    const w = mountDialog(RenameDialog)
    const vm = w.vm as unknown as { open: (r: unknown) => void }
    vm.open(container)
    await settle()

    const input = document.body.querySelector('input') as HTMLInputElement
    input.value = '   '
    input.dispatchEvent(new Event('input'))
    await settle()

    const buttons = [...document.body.querySelectorAll('button')]
    const confirm = buttons.find((b) => b.textContent?.includes('确定')) as HTMLButtonElement
    confirm.click()
    await settle()

    expect(w.emitted('submit')).toBeUndefined()
    expect(document.body.textContent).toContain('此项为必填项')
    expect(document.body.querySelector('.p-dialog')).not.toBeNull()
  })

  it('提交成功路径：裁剪空白后上抛 (id, name)', async () => {
    const w = mountDialog(RenameDialog)
    const vm = w.vm as unknown as { open: (r: unknown) => void }
    vm.open(container)
    await settle()

    const input = document.body.querySelector('input') as HTMLInputElement
    input.value = '  web2  '
    input.dispatchEvent(new Event('input'))
    await settle()

    const buttons = [...document.body.querySelectorAll('button')]
    const confirm = buttons.find((b) => b.textContent?.includes('确定')) as HTMLButtonElement
    confirm.click()
    await settle()

    expect(w.emitted('submit')?.[0]).toEqual(['c1', 'web2'])
  })

  it('close() 收起弹窗（父级在请求成功后调用）', async () => {
    const w = mountDialog(RenameDialog)
    const vm = w.vm as unknown as { open: (r: unknown) => void; close: () => void }
    vm.open(container)
    await settle()
    expect(document.body.querySelector('.p-dialog')).not.toBeNull()

    vm.close()
    await settle()
    expect(document.body.querySelector('.p-dialog')).toBeNull()
  })
})

describe('CreateNetworkDialog', () => {
  it('open()：三个字段就位，子网占位符提示 CIDR 格式', async () => {
    const w = mountDialog(CreateNetworkDialog)
    ;(w.vm as unknown as { open: () => void }).open()
    await settle()

    const inputs = [...document.body.querySelectorAll('input')] as HTMLInputElement[]
    expect(inputs).toHaveLength(2) // 名称 + 子网（驱动是下拉，不是 input）
    expect(inputs.map((i) => i.placeholder)).toContain('172.28.0.0/16')
    expect(document.body.textContent).toContain('创建网络')
    expect(document.body.textContent).toContain('驱动')
  })

  it('名称为空：不发上抛，出现必填错误', async () => {
    const w = mountDialog(CreateNetworkDialog)
    const vm = w.vm as unknown as { open: () => void }
    vm.open()
    await settle()

    const buttons = [...document.body.querySelectorAll('.fp-modal__footer button')]
    ;(buttons[buttons.length - 1] as HTMLButtonElement).click()
    await settle()

    expect(w.emitted('submit')).toBeUndefined()
    expect(document.body.textContent).toContain('此项为必填项')
  })

  it('提交：上抛三字段（名称未裁剪——裁剪由数据层统一做，避免两处各裁一次）', async () => {
    const w = mountDialog(CreateNetworkDialog)
    const vm = w.vm as unknown as { open: () => void }
    vm.open()
    await settle()

    const inputs = [...document.body.querySelectorAll('input')] as HTMLInputElement[]
    inputs[0].value = 'app-net'
    inputs[0].dispatchEvent(new Event('input'))
    await settle()

    const buttons = [...document.body.querySelectorAll('.fp-modal__footer button')]
    ;(buttons[buttons.length - 1] as HTMLButtonElement).click()
    await settle()

    expect(w.emitted('submit')?.[0][0]).toEqual({
      name: 'app-net',
      driver: 'bridge',
      subnet: '',
    })
  })

  it('closeAndReset()：复位表单（名称清空、驱动回 bridge）——对应原实现「成功才清空」', async () => {
    const w = mountDialog(CreateNetworkDialog)
    const vm = w.vm as unknown as {
      open: () => void
      closeAndReset: () => void
    }
    vm.open()
    await settle()

    const inputs = [...document.body.querySelectorAll('input')] as HTMLInputElement[]
    inputs[0].value = 'app-net'
    inputs[0].dispatchEvent(new Event('input'))
    await settle()

    vm.closeAndReset()
    await settle()
    vm.open()
    await settle()

    const reopened = document.body.querySelector('input') as HTMLInputElement
    expect(reopened.value).toBe('')
  })
})

describe('CreateVolumeDialog', () => {
  it('open()：驱动默认 local（卷的白名单与网络不同）', async () => {
    const w = mountDialog(CreateVolumeDialog)
    ;(w.vm as unknown as { open: () => void }).open()
    await settle()
    expect(document.body.textContent).toContain('创建卷')
  })

  it('名称为空拦截 + 成功上抛两字段', async () => {
    const w = mountDialog(CreateVolumeDialog)
    const vm = w.vm as unknown as { open: () => void }
    vm.open()
    await settle()

    const footerBtn = () =>
      [...document.body.querySelectorAll('.fp-modal__footer button')].pop() as HTMLButtonElement

    footerBtn().click()
    await settle()
    expect(w.emitted('submit')).toBeUndefined()

    const input = document.body.querySelector('input') as HTMLInputElement
    input.value = 'data-1'
    input.dispatchEvent(new Event('input'))
    await settle()
    footerBtn().click()
    await settle()

    expect(w.emitted('submit')?.[0][0]).toEqual({ name: 'data-1', driver: 'local' })
  })
})

describe('ConnectContainerDialog', () => {
  const network = { id: 'net1', name: 'app-net', containers: [] }

  it('open(row)：容器候选经 props 注入（父级传容器列表快照）', async () => {
    const w = mountDialog(ConnectContainerDialog, {
      options: [
        { label: 'web', value: 'c1' },
        { label: 'db', value: 'c2' },
      ],
    })
    ;(w.vm as unknown as { open: (r: unknown) => void }).open(network)
    await settle()
    expect(document.body.textContent).toContain('连接容器')
  })

  it('未选容器：本地拦一道（红字），不发上抛', async () => {
    const w = mountDialog(ConnectContainerDialog, { options: [] })
    const vm = w.vm as unknown as { open: (r: unknown) => void }
    vm.open(network)
    await settle()

    const btn = [
      ...document.body.querySelectorAll('.fp-modal__footer button'),
    ].pop() as HTMLButtonElement
    btn.click()
    await settle()

    expect(w.emitted('submit')).toBeUndefined()
    expect(document.body.textContent).toContain('此项为必填项')
  })

  it('选择容器后提交：上抛 (网络 ID, 容器 ID) —— 网络 ID 来自打开时的快照', async () => {
    const w = mountDialog(ConnectContainerDialog, { options: [{ label: 'web', value: 'c1' }] })
    const vm = w.vm as unknown as { open: (r: unknown) => void }
    vm.open(network)
    await settle()

    // FpSelect 的下拉浮层在 jsdom 下不可点选，按组件公开契约直接驱动受控值
    // （这正是真实选中动作的产出：update:modelValue）。
    w.findAllComponents({ name: 'FpSelect' })[0].vm.$emit('update:modelValue', 'c1')
    await settle()

    const btn = [
      ...document.body.querySelectorAll('.fp-modal__footer button'),
    ].pop() as HTMLButtonElement
    btn.click()
    await settle()

    expect(w.emitted('submit')?.[0]).toEqual(['net1', 'c1'])
  })
})
