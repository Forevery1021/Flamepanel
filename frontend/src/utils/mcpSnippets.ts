/**
 * AI 客户端接入配置片段（M-AI-1 · Doc/34 §3）。
 *
 * 纯函数：输入面板地址、MCP 端点路径与令牌，输出各客户端可直接粘贴的配置。
 * 令牌只在签发后的一次性弹窗里出现；列表页用占位符 `fpk_...` 生成示例。
 */

export type McpClientId = 'cursor' | 'claudeCode' | 'claudeDesktop' | 'curl'

export interface McpSnippet {
  client: McpClientId
  /** 代码语言（展示用） */
  lang: 'json' | 'bash'
  code: string
}

/** 占位令牌（未签发时展示示例用） */
export const TOKEN_PLACEHOLDER = 'fpk_...'

/** 客户端配置里的服务名（mcpServers 的键 / `claude mcp add` 的名称） */
export const SERVER_NAME = 'flamepanel'

/** 由页面 origin 与后端返回的 endpoint（`/api/mcp`）拼出完整 URL；endpoint 已是绝对地址时原样返回。 */
export function mcpUrl(origin: string, endpoint = '/api/mcp'): string {
  if (/^https?:\/\//i.test(endpoint)) return endpoint
  const base = origin.replace(/\/+$/, '')
  const path = endpoint.startsWith('/') ? endpoint : `/${endpoint}`
  return `${base}${path}`
}

/** 单引号内安全的 shell 字面量 */
function shQuote(s: string): string {
  return `'${s.replace(/'/g, `'\\''`)}'`
}

function json(value: unknown): string {
  return JSON.stringify(value, null, 2)
}

export function buildMcpSnippets(url: string, token: string = TOKEN_PLACEHOLDER): McpSnippet[] {
  const bearer = `Bearer ${token}`
  return [
    {
      client: 'cursor',
      lang: 'json',
      code: json({ mcpServers: { [SERVER_NAME]: { url, headers: { Authorization: bearer } } } }),
    },
    {
      client: 'claudeCode',
      lang: 'bash',
      code:
        `claude mcp add --transport http ${SERVER_NAME} ${shQuote(url)} \\\n` +
        `  --header ${shQuote(`Authorization: ${bearer}`)}`,
    },
    {
      // stdio 客户端：`flamepanel mcp` 桥（令牌经环境变量，不进 argv）
      client: 'claudeDesktop',
      lang: 'json',
      code: json({
        mcpServers: {
          [SERVER_NAME]: {
            command: 'flamepanel',
            args: ['mcp', '--url', url],
            env: { FLAMEPANEL_MCP_TOKEN: token },
          },
        },
      }),
    },
    {
      client: 'curl',
      lang: 'bash',
      code:
        `curl -s -X POST ${shQuote(url)} \\\n` +
        `  -H ${shQuote(`Authorization: ${bearer}`)} \\\n` +
        `  -H 'Content-Type: application/json' \\\n` +
        `  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"curl","version":"1"}}}'`,
    },
  ]
}

/** 凭证状态：有效 / 已吊销 / 已过期（`active` 由后端计算，这里仅区分失效原因） */
export type PrincipalState = 'active' | 'revoked' | 'expired'

export function principalState(p: { active: boolean; revoked_at?: string | null }): PrincipalState {
  if (p.active) return 'active'
  return p.revoked_at ? 'revoked' : 'expired'
}
