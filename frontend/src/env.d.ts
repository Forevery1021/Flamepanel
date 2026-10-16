/// <reference types="vite/client" />

/* ═══════════ Vue 单文件组件（.vue）模块声明 ═══════════
 * 让 TypeScript 正确识别 `import X from '@/views/xxx.vue'` 等 .vue 导入，
 * 避免 TS7016（隐式 any）——见优化指南 P0 契约化 / 工程化门禁。
 */
declare module '*.vue' {
  import type { DefineComponent } from 'vue'
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const component: DefineComponent<Record<string, unknown>, Record<string, unknown>, any>
  export default component
}
