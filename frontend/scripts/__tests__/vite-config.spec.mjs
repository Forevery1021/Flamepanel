/** A05: exercise native Node/Vite loading, not Vitest's transformed imports. */
import { afterEach, describe, expect, it } from 'vitest'
import { createHash } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

const frontendRoot = resolve(import.meta.dirname, '../..')
const configFile = join(frontendRoot, 'vite.config.ts')
const configUrl = pathToFileURL(configFile).href
const viteUrl = pathToFileURL(createRequire(configFile).resolve('vite')).href
const marker = 'A05_NATIVE_RESULT='
const directories = []

function runNode(source, cwd = frontendRoot) {
  const env = { ...process.env }
  // The regression must pass without the warning-suppression escape hatch.
  delete env.VITE_CONFIG_NATIVE_IGNORE_WARNING
  const result = spawnSync(process.execPath, ['--input-type=module', '-e', source], {
    cwd,
    env,
    encoding: 'utf8',
    timeout: 20000,
    maxBuffer: 2 * 1024 * 1024,
  })
  expect(result.error).toBeUndefined()
  expect(result.status, `${result.stdout}\n${result.stderr}`).toBe(0)
  expect(result.stderr).not.toContain('without a file extension')
  const line = result.stdout.split('\n').find((value) => value.startsWith(marker))
  expect(line, result.stdout).toBeDefined()
  return JSON.parse(line.slice(marker.length))
}

function fixture(html) {
  const root = mkdtempSync(join(tmpdir(), 'flamepanel native vite '))
  directories.push(root)
  writeFileSync(join(root, 'index.html'), html)
  return root
}

afterEach(() => {
  for (const directory of directories.splice(0)) rmSync(directory, { recursive: true, force: true })
})

describe('A05: production Vite config remains native-loader compatible', () => {
  it('Node loads the actual TypeScript config without a bundler or Vitest loader', () => {
    const loaded = runNode(`
      import config from ${JSON.stringify(configUrl)}
      const plugins = config.plugins.flat(Infinity).map((plugin) => plugin?.name).filter(Boolean)
      console.log(${JSON.stringify(marker)} + JSON.stringify({ plugins }))
    `)
    expect(loaded.plugins).toContain('flamepanel-csp')
    expect(loaded.plugins).toContain('vite:vue')
  }, 30000)

  it('Vite can resolve the actual production config with its native loader', () => {
    const loaded = runNode(`
      import { loadConfigFromFile } from ${JSON.stringify(viteUrl)}
      const result = await loadConfigFromFile(
        { command: 'build', mode: 'production' },
        ${JSON.stringify(configFile)}, ${JSON.stringify(frontendRoot)},
        undefined, undefined, 'native',
      )
      if (!result) throw new Error('Production config was not loaded')
      const plugins = result.config.plugins.flat(Infinity).map((plugin) => plugin?.name).filter(Boolean)
      console.log(${JSON.stringify(marker)} + JSON.stringify({ path: result.path, plugins }))
    `)
    expect(loaded.path).toBe(configFile)
    expect(loaded.plugins).toContain('flamepanel-csp')
  }, 30000)

  it('Node loads the Vitest config without CommonJS globals or alias drift', () => {
    const testConfig = pathToFileURL(join(frontendRoot, 'vitest.config.ts')).href
    const loaded = runNode(`
      import config from ${JSON.stringify(testConfig)}
      console.log(${JSON.stringify(marker)} + JSON.stringify({
        alias: config.resolve.alias['@'], include: config.test.include,
      }))
    `)
    expect(loaded.alias).toBe(join(frontendRoot, 'src'))
    expect(loaded.include).toContain('scripts/**/*.spec.mjs')
  }, 30000)

  it('a real native build preserves the inline-script hash and header-only protection', () => {
    const body = "document.documentElement.dataset.a05 = 'native-csp'"
    const root = fixture(
      '<!doctype html><html><head>' +
        '<meta http-equiv="Content-Security-Policy" content="%FLAMEPANEL_CSP%">' +
        `<script>${body}</script></head><body>Native CSP fixture</body></html>`,
    )
    // Run from the isolated root, so relative plugin outputs never touch real source files.
    const built = runNode(
      `
      import { build } from ${JSON.stringify(viteUrl)}
      import { readFileSync } from 'node:fs'
      import { join } from 'node:path'
      const root = ${JSON.stringify(root)}
      await build({
        root, configFile: ${JSON.stringify(configFile)}, configLoader: 'native',
        build: { outDir: join(root, 'dist'), emptyOutDir: true },
      })
      console.log(${JSON.stringify(marker)} + JSON.stringify({
        html: readFileSync(join(root, 'dist/index.html'), 'utf8'),
        snippet: readFileSync(join(root, 'dist/csp.conf'), 'utf8'),
      }))
    `,
      root,
    )
    const hash = createHash('sha256').update(body, 'utf8').digest('base64')
    const expected = [
      "default-src 'self'",
      `script-src 'self' 'sha256-${hash}'`,
      "style-src 'self' 'unsafe-inline'",
      "img-src 'self' data: blob:",
      "font-src 'self' data:",
      "connect-src 'self'",
      "object-src 'none'",
      "base-uri 'self'",
      "form-action 'self'",
    ].join('; ')
    expect(built.html).toContain(`content="${expected}"`)
    expect(built.html).toContain(`<script>${body}</script>`)
    expect(built.html).not.toContain('%FLAMEPANEL_CSP%')
    expect(built.html).not.toContain('frame-ancestors')
    expect(built.snippet).toContain(
      `add_header Content-Security-Policy "${expected}; frame-ancestors 'none'" always;`,
    )
    const scriptDirective = expected.split('; ').find((value) => value.startsWith('script-src '))
    expect(scriptDirective).not.toContain('unsafe-inline')
    expect(scriptDirective).not.toContain('unsafe-eval')
  }, 30000)

  it('a native build still rejects HTML without the mandatory CSP placeholder', () => {
    const root = fixture('<!doctype html><html><head></head><body>Missing CSP</body></html>')
    const rejected = runNode(
      `
      import { build } from ${JSON.stringify(viteUrl)}
      import assert from 'node:assert/strict'
      import { join } from 'node:path'
      await assert.rejects(
        build({
          root: ${JSON.stringify(root)}, configFile: ${JSON.stringify(configFile)}, configLoader: 'native',
          build: { outDir: join(${JSON.stringify(root)}, 'dist'), emptyOutDir: true },
        }),
        /缺少 CSP 占位符/,
      )
      console.log(${JSON.stringify(marker)} + JSON.stringify({ rejectedMissingPlaceholder: true }))
    `,
      root,
    )
    expect(rejected.rejectedMissingPlaceholder).toBe(true)
  }, 30000)
})
