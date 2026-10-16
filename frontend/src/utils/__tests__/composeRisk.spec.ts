import { describe, expect, it } from 'vitest'
import { detectComposeRisk } from '../composeRisk'

const svc = (body: string) => `services:\n  app:\n    image: busybox:1.36\n${body}`

describe('detectComposeRisk', () => {
  it('安全模板无提示', () => {
    expect(detectComposeRisk('')).toBeNull()
    expect(
      detectComposeRisk(
        svc(
          '    volumes:\n      - ./data:/data\n      - appdata:/var/lib/app\n      - /etcetera:/x\n    cap_add: [NET_BIND_SERVICE]\n    restart: always\n',
        ),
      ),
    ).toBeNull()
  })

  it('privileged 各种写法（旧实现只认单空格 `privileged: true`）', () => {
    for (const v of [
      'privileged: true',
      'privileged:  true',
      'privileged: "true"',
      'privileged: yes',
    ]) {
      expect(detectComposeRisk(svc(`    ${v}\n`))).toBe('privileged')
    }
    expect(detectComposeRisk('services: {a: {image: x, privileged: true}}')).toBe('privileged')
  })

  it('docker.sock 任意路径', () => {
    expect(
      detectComposeRisk(svc('    volumes:\n      - /run/docker.sock:/var/run/docker.sock\n')),
    ).toBe('dockerSock')
  })

  it('宿主访问类风险', () => {
    const cases = [
      '    network_mode: host\n',
      '    pid: "host"\n',
      '    cap_add:\n      - SYS_ADMIN\n',
      '    cap_add: [CAP_SYS_PTRACE]\n',
      '    devices:\n      - /dev/sda:/dev/sda\n',
      '    security_opt:\n      - seccomp=unconfined\n',
      '    volumes:\n      - /:/host\n',
      '    volumes:\n      - "/etc:/host/etc:ro"\n',
      '    volumes:\n      - type: bind\n        source: /root\n        target: /r\n',
      '    volumes:\n      - ~/.ssh:/keys\n',
    ]
    for (const c of cases) expect(detectComposeRisk(svc(c)), c).toBe('hostAccess')
  })
})
