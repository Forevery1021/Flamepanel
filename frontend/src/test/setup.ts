import { afterAll } from 'vitest'

/**
 * T1-1 组件测试基建：补齐 jsdom 缺失的浏览器 API。
 *
 * Fp* 组件的底层 OpenVue 实现会直接消费这些 API（Select 读 `matchMedia` 判断
 * 弹层展开方向、TabList 用 `ResizeObserver` 监听溢出、Datatable 虚拟滚动用
 * `IntersectionObserver`），jsdom 未实现其中任何一个，缺失时组件渲染期即抛
 * `TypeError`——组件测试在「挂载」这一步就失败，无从断言行为。
 *
 * 这里提供**最小可用桩**：只满足「存在且可调用」，不模拟真实布局计算（jsdom
 * 没有排版引擎，任何尺寸读取都是 0，强行模拟反而制造假绿）。需要真实尺寸的
 * 断言应放到浏览器环境（E2E，见 Doc/23 §3.2 T1-2）。
 */

if (!window.matchMedia) {
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    value: (query: string): MediaQueryList =>
      ({
        matches: false,
        media: query,
        onchange: null,
        addListener: () => {},
        removeListener: () => {},
        addEventListener: () => {},
        removeEventListener: () => {},
        dispatchEvent: () => false,
      }) as unknown as MediaQueryList,
  })
}

class NoopResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}

class NoopIntersectionObserver {
  root = null
  rootMargin = ''
  thresholds: number[] = []
  observe() {}
  unobserve() {}
  disconnect() {}
  takeRecords() {
    return []
  }
}

if (!('ResizeObserver' in globalThis)) {
  Object.defineProperty(globalThis, 'ResizeObserver', { writable: true, value: NoopResizeObserver })
}

if (!('IntersectionObserver' in globalThis)) {
  Object.defineProperty(globalThis, 'IntersectionObserver', {
    writable: true,
    value: NoopIntersectionObserver,
  })
}

/**
 * 用例文件收尾前等一拍，让第三方组件遗留的短计时器在 jsdom 仍存活时触发。
 *
 * OpenVue TabList 在 `mounted` 里挂了一个 150ms 的 `setTimeout`（inkbar 定位）且卸载时
 * 不清理；若用例文件在它触发前结束，回调会在环境销毁后引用 `HTMLElement` 抛
 * ReferenceError——vitest 记为 Unhandled Error，云端门禁曾因此偶发红（用例本身全绿）。
 * 环境存活时触发则安全：组件已卸载，`$refs` 为空，回调直接返回。
 */
afterAll(async () => {
  await new Promise((resolve) => setTimeout(resolve, 200))
})
