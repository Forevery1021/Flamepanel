import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import { npmVersionFromAgent, toolchainErrors } from './check-toolchain.mjs'

const scriptDir = dirname(fileURLToPath(import.meta.url))
const expectedNode = readFileSync(resolve(scriptDir, '../../.node-version'), 'utf8').trim()
const { packageManager } = JSON.parse(readFileSync(resolve(scriptDir, '../package.json'), 'utf8'))
const expectedNpm = packageManager.slice(4)
const correct = { nodeVersion: expectedNode, npmVersion: expectedNpm, expectedNode, expectedNpm }

describe('frontend toolchain contract', () => {
  it('accepts the pinned Node/npm pair', () => {
    expect(toolchainErrors(correct)).toEqual([])
  })

  it('rejects a different Node version', () => {
    expect(toolchainErrors({ ...correct, nodeVersion: '0.0.0' })).toHaveLength(1)
  })

  it('rejects a different npm version', () => {
    expect(toolchainErrors({ ...correct, npmVersion: '0.0.0' })).toHaveLength(1)
  })

  it('reports both missing versions', () => {
    expect(
      toolchainErrors({ ...correct, nodeVersion: undefined, npmVersion: undefined }),
    ).toHaveLength(2)
  })

  it('reads the npm user agent without assuming the bundled Node version', () => {
    expect(npmVersionFromAgent(`npm/${expectedNpm} node/v${expectedNode} linux x64`)).toBe(
      expectedNpm,
    )
  })

  it('rejects pnpm and a missing user agent', () => {
    expect(npmVersionFromAgent('pnpm/10.0.0 npm/? node/v24.0.0')).toBeUndefined()
    expect(npmVersionFromAgent()).toBeUndefined()
  })

  it('pins a valid npm packageManager', () => {
    expect(packageManager).toMatch(/^npm@\d+\.\d+\.\d+$/)
  })

  it.each(['../../Dockerfile', '../../.ide/Dockerfile'])('keeps %s Node image aligned', (path) => {
    const dockerfile = readFileSync(resolve(scriptDir, path), 'utf8')
    expect(dockerfile).toContain(`FROM node:${expectedNode}-bookworm-slim AS`)
  })
})
