import { mount, type VueWrapper } from '@vue/test-utils'
import type { Component, Plugin } from 'vue'
import { createI18n } from 'vue-i18n'
import OpenVue from 'openvue/config'
import ConfirmationService from 'openvue/confirmationservice'
import ToastService from 'openvue/toastservice'
import flamePreset from '@/theme/flame-preset'

/**
 * T1-1 组件测试公共挂载层。
 *
 * 为什么需要它：Fp* 组件的底层 OpenVue 实现从 `$primevue`（config 插件）读取
 * 主题与语言包配置，未安装插件时渲染期访问 `$primevue` 会抛 TypeError——这是
 * 「组件测试从零起步」的第一个拦路虎，统一在此处解决。
 *
 * 主题预设与 `src/main.ts` 保持一致（flamePreset）：组件在测试中拿到的 class /
 * severity 映射与生产一致，避免「测试里能过、线上样式错位」。
 */

/** 测试用 i18n：默认装载组件文案常用 key，可按用例覆盖 */
export function createTestI18n(messages: Record<string, unknown> = {}) {
  return createI18n({
    legacy: false,
    locale: 'zh-CN',
    fallbackWarn: false,
    missingWarn: false,
    messages: {
      'zh-CN': {
        common: {
          refresh: '刷新',
          searchPlaceholder: '搜索',
          copy: '复制',
          copySuccess: '已复制',
          copyFailed: '复制失败',
          failed: '操作失败',
          retry: '重试',
          noData: '暂无数据',
        },
        ...messages,
      },
    },
  })
}

/**
 * 全局服务：`useFpConfirm` / `useFpToast` 通过 inject 取用，与 main.ts 保持同源。
 * 两个服务只挂 `$confirm` / `$toast` 到 app 上，不渲染任何节点，可安全默认安装。
 */
export const services = [ConfirmationService, ToastService] as unknown as Plugin[]

/** 与 main.ts 同源的 OpenVue config 插件（tuple 形式，供 global.plugins 使用） */
export const openVuePlugin = [
  OpenVue,
  { theme: { preset: flamePreset, options: { darkModeSelector: '.dark', cssLayer: false } } },
] as unknown as Plugin

/** Fp* 组件普遍使用 `v-model` 等自定义 props，测试侧统一以宽松签名挂载 */
export interface FpMountOptions {
  props?: Record<string, unknown>
  slots?: Record<string, string | (() => unknown)>
  attrs?: Record<string, unknown>
  attachTo?: Element | string
  messages?: Record<string, unknown>
  global?: Record<string, unknown>
  [key: string]: unknown
}

/**
 * 统一挂载入口：默认注入 OpenVue config + 测试 i18n，可按需追加/覆盖。
 *
 * 泛型放宽为 `Component`：Fp* 组件普遍用 `defineModel` / 泛型 props，精确推断
 * 会把每个用例的 props 形状绑死，反而让测试代码比被测组件更难维护——这里主动
 * 选择宽松签名（类型安全由各组件的 `defineProps` 声明与 typecheck 保障）。
 */
export function mountFp(
  component: Component,
  options: FpMountOptions = {},
): VueWrapper<Record<string, unknown>> {
  const { messages, global: globalOptions, ...rest } = options
  // 走一次宽松 cast：@vue/test-utils 的 mount 泛型会按组件 props 精确约束 options，
  // 而本层刻意接受任意 props（见上），故在此收口（仅此一处 unsound）。
  const mountLoose = mount as unknown as (
    c: Component,
    o: Record<string, unknown>,
  ) => VueWrapper<Record<string, unknown>>
  return mountLoose(component, {
    ...rest,
    global: {
      plugins: [createTestI18n(messages), openVuePlugin, ...services],
      ...globalOptions,
    },
  })
}

/** 挂载宿主元素：用于 Teleport 到 body 的浮层组件（Modal / Drawer / Select 弹层） */
export function createHost(): HTMLDivElement {
  const host = document.createElement('div')
  document.body.appendChild(host)
  return host
}
