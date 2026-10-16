import { describe, it, expect } from 'vitest'
import { buildMcpSnippets, mcpUrl, principalState, TOKEN_PLACEHOLDER } from '@/utils/mcpSnippets'

/** M-AI-1 PR-B：AI 客户端接入片段（Doc/34 §3）——结构可解析、令牌不进 argv、shell 引号安全。 */
describe('mcpSnippets', () => {
  const url = 'https://panel.example.com/api/mcp'
  const token = 'fpk_' + 'a'.repeat(64)

  it('mcpUrl 拼接 origin 与 endpoint，容忍多余斜杠与绝对地址', () => {
    expect(mcpUrl('https://panel.example.com/', '/api/mcp')).toBe(url)
    expect(mcpUrl('https://panel.example.com', 'api/mcp')).toBe(url)
    expect(mcpUrl('https://panel.example.com')).toBe(url)
    expect(mcpUrl('https://x', 'https://other/api/mcp')).toBe('https://other/api/mcp')
  })

  it('覆盖四个客户端，缺省使用占位令牌', () => {
    const s = buildMcpSnippets(url)
    expect(s.map((x) => x.client)).toEqual(['cursor', 'claudeCode', 'claudeDesktop', 'curl'])
    for (const x of s) expect(x.code).toContain(TOKEN_PLACEHOLDER)
  })

  it('Cursor：Streamable HTTP + Authorization 头', () => {
    const cursor = buildMcpSnippets(url, token).find((x) => x.client === 'cursor')!
    const cfg = JSON.parse(cursor.code)
    expect(cfg.mcpServers.flamepanel).toEqual({ url, headers: { Authorization: `Bearer ${token}` } })
  })

  it('Claude Desktop：经 flamepanel mcp stdio 桥，令牌只走环境变量', () => {
    const d = buildMcpSnippets(url, token).find((x) => x.client === 'claudeDesktop')!
    const srv = JSON.parse(d.code).mcpServers.flamepanel
    expect(srv.command).toBe('flamepanel')
    expect(srv.args).toEqual(['mcp', '--url', url])
    expect(srv.args.join(' ')).not.toContain(token)
    expect(srv.env).toEqual({ FLAMEPANEL_MCP_TOKEN: token })
  })

  it('Claude Code / curl：单引号包裹，URL 中的单引号被转义', () => {
    const odd = "https://p.example.com/it's/api/mcp"
    const s = buildMcpSnippets(odd, token)
    const cc = s.find((x) => x.client === 'claudeCode')!.code
    expect(cc).toContain('claude mcp add --transport http flamepanel')
    expect(cc).toContain(`'https://p.example.com/it'\\''s/api/mcp'`)
    expect(cc).toContain(`--header 'Authorization: Bearer ${token}'`)
    const curl = s.find((x) => x.client === 'curl')!.code
    expect(curl).toContain('"method":"initialize"')
    expect(curl).toContain(`-H 'Authorization: Bearer ${token}'`)
  })

  it('principalState 区分吊销与过期', () => {
    expect(principalState({ active: true })).toBe('active')
    expect(principalState({ active: false, revoked_at: '2026-01-01' })).toBe('revoked')
    expect(principalState({ active: false, revoked_at: null })).toBe('expired')
  })
})
