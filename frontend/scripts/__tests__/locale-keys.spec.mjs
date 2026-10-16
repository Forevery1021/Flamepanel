/**
 * i18n key 门禁单测（T2-5，Doc/23 §3.3 / §7.24）。
 *
 * 门禁脚本算错会「假绿」（放行裸 key 上屏）或「假红」（误拦合法提交），
 * 故用真实临时目录构造语言包夹具做端到端验证；并另起一组用例直接校验
 * 本仓真实语言包（拆分后的结构一致性 / 三语言 key 相等 / 引用全覆盖）。
 */
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import {
  LocaleKeysError,
  checkLocaleKeys,
  flattenPack,
  placeholders,
  run,
  scanSource,
} from '../check-locale-keys.mjs'

const LANGS = ['zh-CN', 'en-US', 'ja-JP']

/** 构造一套语言包夹具：{ 'zh-CN': { common: { save: '保存' } }, ... } */
function makeFixture(packs, { srcFiles = {}, modules } = {}) {
  const root = mkdtempSync(join(tmpdir(), 'locale-gate-'))
  const localesDir = join(root, 'locales')
  const srcDir = join(root, 'src')
  mkdirSync(srcDir, { recursive: true })
  for (const [lang, tree] of Object.entries(packs)) {
    mkdirSync(join(localesDir, lang), { recursive: true })
    const modNames = modules ?? Object.keys(tree)
    for (const mod of modNames) {
      const body = tree[mod] ?? {}
      // 夹具里一个模块承载一个命名空间，便于按模块清单核对结构一致性
      writeFileSync(
        join(localesDir, lang, `${mod}.ts`),
        `export default {\n  ${mod}: ${JSON.stringify(body, null, 2).replace(/\n/g, '\n  ')},\n}\n`,
      )
    }
    writeFileSync(
      join(localesDir, lang, 'index.ts'),
      modNames.map((m) => `import ${m} from './${m}'`).join('\n') +
        `\n\nexport default {\n${modNames.map((m) => `  ...${m},`).join('\n')}\n}\n`,
    )
  }
  for (const [name, content] of Object.entries(srcFiles)) {
    const full = join(srcDir, name)
    mkdirSync(join(full, '..'), { recursive: true })
    writeFileSync(full, content)
  }
  return root
}

let roots = []
const bind = (root) => (roots.push(root), root)
beforeEach(() => {
  roots = []
})
afterEach(() => {
  for (const root of roots) rmSync(root, { recursive: true, force: true })
})

describe('flattenPack / placeholders：纯函数', () => {
  it('嵌套对象扁平化为 a.b.c，叶子保持原值', () => {
    const flat = flattenPack({ a: { b: { c: 'x' }, d: 'y' }, e: 'z' })
    expect([...flat.keys()].sort()).toEqual(['a.b.c', 'a.d', 'e'])
    expect(flat.get('a.b.c')).toBe('x')
  })

  it('同一命名空间被两个模块重复导出时报错（后写覆盖前写 = 静默丢文案）', () => {
    const root = bind(
      makeFixture(
        {
          'zh-CN': { common: { save: '保存' }, shell: { save: '另存' } },
          'en-US': { common: { save: 'Save' }, shell: { save: 'Save' } },
          'ja-JP': { common: { save: '保存' }, shell: { save: '保存' } },
        },
        { modules: ['common', 'shell'] },
      ),
    )
    // 把 ja-JP/shell.ts 的内容改成重复导出 common 命名空间
    writeFileSync(
      join(root, 'locales', 'ja-JP', 'shell.ts'),
      "export default {\n  common: { save: '保存' },\n}\n",
    )
    const res = checkLocaleKeys({ localesDir: join(root, 'locales'), srcDir: join(root, 'src') })
    expect(res.ok).toBe(false)
    expect(res.problems.join('\n')).toContain('重复定义命名空间')
  })

  it('占位符提取：多参数排序、同参数去重、无参数为空数组', () => {
    expect(placeholders('确定删除 {name} 吗？共 {count} 项')).toEqual(['count', 'name'])
    expect(placeholders('删除 {name}，{name} 关联的 {env}')).toEqual(['env', 'name'])
    expect(placeholders('普通文案')).toEqual([])
  })
})

describe('scanSource：源码 key 引用扫描', () => {
  it('识别 t / te / tt 的静态 key 与行号', () => {
    const src = `const a = t('common.save')\nconst b = te("ns.key")\nconst c = tt('x.y', '兜底')\n`
    const { statics, prefixes } = scanSource('x.ts', src)
    expect(statics.map((s) => s.key)).toEqual(['common.save', 'ns.key', 'x.y'])
    expect(statics.map((s) => s.line)).toEqual([1, 2, 3])
    expect(prefixes).toEqual([])
  })

  it('识别动态拼接前缀（字符串拼接与模板字符串两种形态）', () => {
    const src = "const a = t('certificate.source_' + s)\nconst b = t(`settingsTheme.${p}`)\n"
    const { statics, prefixes } = scanSource('x.ts', src)
    expect(statics).toEqual([])
    expect(prefixes.map((p) => p.key)).toEqual(['certificate.source_', 'settingsTheme.'])
  })

  it('识别路由 meta.title 形态（侧栏/页签/⌘K 三处消费的同一 key）', () => {
    const src = `meta: { title: 'nav.dashboard', icon: 'oi-gauge' }`
    const { statics } = scanSource('router.ts', src)
    expect(statics.map((s) => s.key)).toEqual(['nav.dashboard'])
  })

  it('不误报对象属性名与非 i18n 字面量（obj.t 形式 / 普通资产路径）', () => {
    const src = `this.t('不是i18n')\nconst url = '/assets/app.js'\nconst x = app.config.errorHandler\n`
    const { statics, prefixes } = scanSource('x.ts', src)
    expect(statics.map((s) => s.key)).toEqual([])
    expect(prefixes).toEqual([])
  })
})

describe('checkLocaleKeys：结构一致性（A）', () => {
  it('三语言模块清单一致时通过', () => {
    const root = bind(
      makeFixture({
        'zh-CN': { common: { save: '保存' } },
        'en-US': { common: { save: 'Save' } },
        'ja-JP': { common: { save: '保存' } },
      }),
    )
    const res = checkLocaleKeys({ localesDir: join(root, 'locales'), srcDir: join(root, 'src') })
    expect(res.ok).toBe(true)
    expect(res.stats.keysPerLang).toEqual({ 'zh-CN': 1, 'en-US': 1, 'ja-JP': 1 })
  })

  it('某语言缺模块时报出模块名（切语言整块裸 key 的事故形态）', () => {
    const root = bind(
      makeFixture(
        {
          'zh-CN': { common: { save: '保存' }, docker: { start: '启动' } },
          'en-US': { common: { save: 'Save' }, docker: { start: 'Start' } },
          'ja-JP': { common: { save: '保存' } },
        },
        { modules: ['common', 'docker'] },
      ),
    )
    // ja-JP 目录只写 common：手动删掉 docker
    rmSync(join(root, 'locales', 'ja-JP', 'docker.ts'))
    writeFileSync(
      join(root, 'locales', 'ja-JP', 'index.ts'),
      "import common from './common'\n\nexport default {\n  ...common,\n}\n",
    )
    const res = checkLocaleKeys({ localesDir: join(root, 'locales'), srcDir: join(root, 'src') })
    expect(res.ok).toBe(false)
    expect(res.problems.join('\n')).toContain('ja-JP 缺少模块：docker')
  })
})

describe('checkLocaleKeys：key 集合与占位符（B/C）', () => {
  it('某语言缺 key 时精确报出 key 名', () => {
    const root = bind(
      makeFixture({
        'zh-CN': { common: { save: '保存', delete: '删除' } },
        'en-US': { common: { save: 'Save' } },
        'ja-JP': { common: { save: '保存', delete: '削除' } },
      }),
    )
    const res = checkLocaleKeys({ localesDir: join(root, 'locales'), srcDir: join(root, 'src') })
    expect(res.ok).toBe(false)
    expect(res.problems.join('\n')).toContain('key「common.delete」在 en-US 缺失')
  })

  it('占位符不一致时报出双方 token（漏译参数会让句子残缺）', () => {
    const root = bind(
      makeFixture({
        'zh-CN': { common: { del: '删除 {name}' } },
        'en-US': { common: { del: 'Delete' } },
        'ja-JP': { common: { del: '{name} を削除' } },
      }),
    )
    const res = checkLocaleKeys({ localesDir: join(root, 'locales'), srcDir: join(root, 'src') })
    expect(res.ok).toBe(false)
    expect(res.problems.join('\n')).toContain('key「common.del」占位符不一致')
    expect(res.problems.join('\n')).toContain('zh-CN=[name] en-US=[]')
  })

  it('空串值报错（界面上会渲染空白）', () => {
    const root = bind(
      makeFixture({
        'zh-CN': { common: { blank: ' ' } },
        'en-US': { common: { blank: 'x' } },
        'ja-JP': { common: { blank: 'x' } },
      }),
    )
    const res = checkLocaleKeys({ localesDir: join(root, 'locales'), srcDir: join(root, 'src') })
    expect(res.ok).toBe(false)
    expect(res.problems.join('\n')).toContain('key「common.blank」为空串')
  })
})

describe('checkLocaleKeys：源码引用（D）', () => {
  it('源码引用不存在于某语言的 key 时报出文件与行号', () => {
    const root = bind(
      makeFixture(
        {
          'zh-CN': { common: { save: '保存' } },
          'en-US': { common: { save: 'Save' } },
          'ja-JP': { common: { save: '保存' } },
        },
        {
          srcFiles: {
            'views/A.vue': `const a = t('common.save')\nconst b = t('common.missing')\n`,
          },
        },
      ),
    )
    const res = checkLocaleKeys({ localesDir: join(root, 'locales'), srcDir: join(root, 'src') })
    expect(res.ok).toBe(false)
    expect(res.problems.join('\n')).toContain('common.missing')
    expect(res.problems.join('\n')).toContain('views/A.vue:2')
  })

  it('动态前缀下一条 key 都不命中时报错；命中则通过', () => {
    const root = bind(
      makeFixture(
        {
          'zh-CN': { s: { a_x: '甲' } },
          'en-US': { s: { a_x: 'A' } },
          'ja-JP': { s: { a_x: 'ア' } },
        },
        { srcFiles: { 'x.ts': `t('s.a_' + k)\n` } },
      ),
    )
    expect(
      checkLocaleKeys({ localesDir: join(root, 'locales'), srcDir: join(root, 'src') }).ok,
    ).toBe(true)
    writeFileSync(join(root, 'src', 'x.ts'), "t('s.zz_' + k)\n")
    const res = checkLocaleKeys({ localesDir: join(root, 'locales'), srcDir: join(root, 'src') })
    expect(res.ok).toBe(false)
    expect(res.problems.join('\n')).toContain('动态前缀「s.zz_」')
  })

  it('生成物 / 测试 / 语言包自身不参与引用扫描（否则必然自红）', () => {
    const root = bind(
      makeFixture(
        {
          'zh-CN': { common: { save: '保存' } },
          'en-US': { common: { save: 'Save' } },
          'ja-JP': { common: { save: '保存' } },
        },
        {
          srcFiles: {
            'api/generated/openapi.ts': `t('not.a.real.key')\n`,
            'utils/__tests__/a.spec.ts': `t('also.not.real')\n`,
          },
        },
      ),
    )
    expect(
      checkLocaleKeys({ localesDir: join(root, 'locales'), srcDir: join(root, 'src') }).ok,
    ).toBe(true)
  })
})

describe('run：CLI 输出与退出语义', () => {
  it('全绿时打印统计并返回结果', () => {
    const root = bind(
      makeFixture({
        'zh-CN': { common: { save: '保存' } },
        'en-US': { common: { save: 'Save' } },
        'ja-JP': { common: { save: '保存' } },
      }),
    )
    const logs = []
    const res = run({
      localesDir: join(root, 'locales'),
      srcDir: join(root, 'src'),
      log: (l) => logs.push(l),
    })
    expect(res.ok).toBe(true)
    expect(logs.join('\n')).toContain('语言模块：1 个模块 × 3 语言')
    expect(logs.join('\n')).toContain('i18n key 一致性校验通过')
  })

  it('有差异时抛 LocaleKeysError（CI 退出码 1）并附问题清单', () => {
    const root = bind(
      makeFixture({
        'zh-CN': { common: { save: '保存' } },
        'en-US': { common: {} },
        'ja-JP': { common: { save: '保存' } },
      }),
    )
    let err
    try {
      run({ localesDir: join(root, 'locales'), srcDir: join(root, 'src'), log: () => {} })
    } catch (e) {
      err = e
    }
    expect(err).toBeInstanceOf(LocaleKeysError)
    expect(err.problems.length).toBeGreaterThan(0)
    expect(err.message).toContain('i18n key 一致性校验未通过')
  })
})

describe('真实语言包：本仓 src/locales 现状（T2-5 交付后基线）', () => {
  const localesDir = 'src/locales'
  const srcDir = 'src'

  it('三语言模块清单一致，且与聚合入口 imports 一致', () => {
    const idx = readFileSync(join(localesDir, 'zh-CN', 'index.ts'), 'utf8')
    const mods = [...idx.matchAll(/from '\.\/([\w-]+)'/g)].map((m) => m[1])
    expect(mods.length).toBeGreaterThanOrEqual(20)
    for (const lang of LANGS) {
      const other = readFileSync(join(localesDir, lang, 'index.ts'), 'utf8')
      expect([...other.matchAll(/from '\.\/([\w-]+)'/g)].map((m) => m[1])).toEqual(mods)
    }
  })

  it('真实语言包通过门禁（三语言 key 集合一致 + 引用全部命中）', () => {
    const res = checkLocaleKeys({ localesDir, srcDir })
    expect(res.problems).toEqual([])
    expect(res.ok).toBe(true)
  })

  it('拆分后无千行单体：单文件行数全部低于 200（P1-6 的判据）', () => {
    for (const lang of LANGS) {
      const idx = readFileSync(join(localesDir, lang, 'index.ts'), 'utf8')
      for (const m of idx.matchAll(/from '\.\/([\w-]+)'/g)) {
        const lines = readFileSync(join(localesDir, lang, `${m[1]}.ts`), 'utf8').split('\n').length
        expect(lines, `${lang}/${m[1]}.ts 行数`).toBeLessThan(200)
      }
    }
  })
})
