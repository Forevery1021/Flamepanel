import { describe, it, expect } from 'vitest'

/**
 * a11y 静态审计守卫（batch-10）。仓库未引入 axe（不新增依赖），用模板静态扫描兜住两类
 * 最常见、读屏 / 键盘用户直接无法使用的回归：
 *
 * 1. **icon-only 按钮无可访问名称**：自闭合 `<FpButton icon=…/>`（或原生 `<button>` 内只有 `<i>`）
 *    却没有 `aria-label` / `label`——`title` 只是提示，不作为可靠名称；
 * 2. **可点击的非交互元素**：`<div|span|li|td|tr @click>` 缺 `role` + `tabindex`，键盘不可达
 *    （`role="option"` 走 aria-activedescendant，豁免）。
 *
 * 白名单仅收「同一操作另有可聚焦按钮」的鼠标便捷区域，新增须写明理由。
 */
const sources = import.meta.glob(['../**/*.vue', '!../**/__tests__/**'], {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>

const ALLOW_CLICKABLE: Record<string, string> = {
  // 推荐卡片整块可点只是鼠标便捷，卡片内已有同操作的「安装」FpButton
  'components/appStore/AppStoreGrid.vue': 'recommend-card',
}

const ATTRS = String.raw`((?:[^>"']|"[^"]*"|'[^']*')*?)`
const HAS_NAME = /(?<![\w-]):?(aria-label|aria-labelledby|label)\s*=|v-bind\s*=/

function template(src: string) {
  const start = src.indexOf('<template')
  return start < 0 ? '' : src.slice(start, src.lastIndexOf('</template>'))
}
const rel = (path: string) => path.replace(/^\.\.\//, '')

describe('a11y 静态审计', () => {
  it('扫描到足量组件（防 glob 失效导致空跑）', () => {
    expect(Object.keys(sources).length).toBeGreaterThan(150)
  })

  it('icon-only 按钮都有可访问名称', () => {
    const offenders: string[] = []
    const fpButton = new RegExp(String.raw`<(?:FpButton|Button)\b${ATTRS}/>`, 'g')
    const nativeIconButton = new RegExp(
      String.raw`<button\b${ATTRS}>\s*<i\b[^>]*(?:/>|>\s*</i>)\s*</button>`,
      'g',
    )
    for (const [path, src] of Object.entries(sources)) {
      const tpl = template(src)
      for (const m of tpl.matchAll(fpButton)) {
        const attrs = m[1] ?? ''
        if (/(?<![\w-]):?icon\s*=/.test(attrs) && !HAS_NAME.test(attrs)) offenders.push(rel(path))
      }
      for (const m of tpl.matchAll(nativeIconButton)) {
        if (!HAS_NAME.test(m[1] ?? '')) offenders.push(rel(path))
      }
    }
    expect(offenders).toEqual([])
  })

  it('可点击的非交互元素具备 role + tabindex（键盘可达）', () => {
    const offenders: string[] = []
    const clickable = new RegExp(String.raw`<(div|span|li|td|tr)\b${ATTRS}>`, 'gs')
    for (const [path, src] of Object.entries(sources)) {
      const file = rel(path)
      for (const m of template(src).matchAll(clickable)) {
        const attrs = m[2] ?? ''
        if (!/@click(?![.\w]*\.(?:self|stop)\b)/.test(attrs)) continue
        if (/@click\.(?:self|stop)\b/.test(attrs)) continue
        if (/(?<![\w-]):?role\s*=/.test(attrs) && /(?<![\w-]):?tabindex\s*=/.test(attrs)) continue
        // listbox 选项走 aria-activedescendant（焦点留在输入框），无需自身可聚焦
        if (/(?<![\w-])role="option"/.test(attrs)) continue
        const allowed = ALLOW_CLICKABLE[file]
        if (allowed && attrs.includes(allowed)) continue
        offenders.push(`${file} <${m[1]} ${attrs.trim().split(/\s+/).slice(0, 2).join(' ')}>`)
      }
    }
    expect(offenders).toEqual([])
  })
})
