import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

export function toolchainErrors({ nodeVersion, npmVersion, expectedNode, expectedNpm }) {
  const errors = []
  if (nodeVersion !== expectedNode) {
    errors.push(`Node ${nodeVersion ?? 'unknown'} != ${expectedNode} (.node-version)`)
  }
  if (npmVersion !== expectedNpm) {
    errors.push(`npm ${npmVersion ?? 'unknown'} != ${expectedNpm} (packageManager)`)
  }
  return errors
}

export function npmVersionFromAgent(agent = '') {
  return /^npm\/([^\s]+)/.exec(agent)?.[1]
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const expectedNode = readFileSync(new URL('../../.node-version', import.meta.url), 'utf8').trim()
  const { packageManager } = JSON.parse(
    readFileSync(new URL('../package.json', import.meta.url), 'utf8'),
  )
  if (!/^npm@\d+\.\d+\.\d+$/.test(packageManager ?? '')) {
    console.error('packageManager must pin npm@<major>.<minor>.<patch>')
    process.exitCode = 1
  } else {
    const expectedNpm = packageManager.slice(4)
    const errors = toolchainErrors({
      nodeVersion: process.versions.node,
      npmVersion: npmVersionFromAgent(process.env.npm_config_user_agent),
      expectedNode,
      expectedNpm,
    })
    if (errors.length) {
      console.error(errors.join('\n'))
      console.error(
        `Use Node ${expectedNode}, install npm@${expectedNpm}, then npm run check:toolchain.`,
      )
      process.exitCode = 1
    } else {
      console.log(`Toolchain OK: Node ${expectedNode}, npm ${expectedNpm}`)
    }
  }
}
