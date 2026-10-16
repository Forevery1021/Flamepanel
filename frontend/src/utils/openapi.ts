/**
 * OpenAPI 文档的纯函数工具（面板内 API 文档页，Doc/32 §6）。
 *
 * 与 `scripts/gen-api-docs.py` 同一套规则（类型名、示例生成、权限文案），
 * 保证面板里看到的与 `Doc/api/*.md` 一致。只依赖文档本身，不访问网络，可离线单测。
 */

export type HttpMethod = 'get' | 'post' | 'put' | 'patch' | 'delete'
export const HTTP_METHODS: readonly HttpMethod[] = ['get', 'post', 'put', 'patch', 'delete']

/** OpenAPI Schema 子集（只声明本页用到的字段） */
export interface Schema {
  $ref?: string
  type?: string | string[]
  format?: string
  enum?: unknown[]
  items?: Schema
  properties?: Record<string, Schema>
  required?: string[]
  additionalProperties?: boolean | Schema
  oneOf?: Schema[]
  anyOf?: Schema[]
  allOf?: Schema[]
  description?: string
  example?: unknown
  default?: unknown
}

interface MediaType {
  schema?: Schema
}

export interface OpenApiParameter {
  name: string
  in: string
  required?: boolean
  description?: string
  schema?: Schema
}

interface RawOperation {
  operationId?: string
  summary?: string
  description?: string
  tags?: string[]
  parameters?: OpenApiParameter[]
  requestBody?: { required?: boolean; content?: Record<string, MediaType> }
  responses?: Record<string, { description?: string; content?: Record<string, MediaType> }>
  security?: unknown[]
  'x-permission'?: string
}

export interface OpenApiDocument {
  openapi?: string
  info?: { title?: string; version?: string; description?: string }
  paths?: Record<string, Partial<Record<HttpMethod, RawOperation>>>
  components?: { schemas?: Record<string, Schema> }
}

export interface ApiResponseInfo {
  status: string
  description: string
  contentType?: string
  schema?: Schema
}

export interface ApiOperation {
  /** 稳定键：`get /api/users` */
  key: string
  method: HttpMethod
  path: string
  tag: string
  operationId: string
  summary: string
  description: string
  /** `resource:action`；空串 = 未登记细粒度权限 */
  permission: string
  secured: boolean
  parameters: OpenApiParameter[]
  requestBody?: { contentType: string; required: boolean; schema?: Schema }
  responses: ApiResponseInfo[]
}

export interface SchemaField {
  name: string
  type: string
  required: boolean
  description: string
}

function refName(ref: string): string {
  return ref.slice(ref.lastIndexOf('/') + 1)
}

/** 展开 `$ref`（带环保护） */
export function resolveSchema(doc: OpenApiDocument, schema: Schema | undefined): Schema {
  let s = schema
  for (let i = 0; i < 16 && s?.$ref; i++) {
    s = doc.components?.schemas?.[refName(s.$ref)]
  }
  return s ?? {}
}

function nonNullType(t: Schema['type']): { type?: string; nullable: boolean } {
  if (Array.isArray(t)) {
    return { type: t.find((x) => x !== 'null'), nullable: t.includes('null') }
  }
  return { type: t, nullable: false }
}

/** 人类可读的类型名：`User`、`string(date-time)`、`Memo[]`、`map<string, any>`，可空加 `?` */
export function typeName(schema: Schema | undefined): string {
  if (!schema || Object.keys(schema).length === 0) return 'any'
  if (schema.$ref) return refName(schema.$ref)
  const union = schema.oneOf ?? schema.anyOf
  if (union) {
    const parts = union.map(typeName)
    const nullable = parts.includes('null')
    return parts.filter((p) => p !== 'null').join(' | ') + (nullable ? '?' : '')
  }
  if (schema.allOf) return schema.allOf.map(typeName).join(' & ')
  const { type, nullable } = nonNullType(schema.type)
  let name: string
  if (type === 'array') {
    name = `${typeName(schema.items)}[]`
  } else if (schema.enum) {
    name = schema.enum
      .filter((v) => v !== null)
      .map((v) => JSON.stringify(v))
      .join(' | ')
  } else if (type === 'object' && schema.additionalProperties && !schema.properties) {
    const ap = schema.additionalProperties
    name = `map<string, ${typeof ap === 'object' ? typeName(ap) : 'any'}>`
  } else {
    name = type ?? 'any'
    if (schema.format) name += `(${schema.format})`
  }
  return nullable ? `${name}?` : name
}

const STRING_FORMATS: Record<string, string> = {
  'date-time': '2026-01-01T00:00:00Z',
  date: '2026-01-01',
  uuid: '00000000-0000-0000-0000-000000000000',
  binary: '<binary>',
}

/** 由 Schema 生成示例值（优先 `example` / `enum` / `default`，递归深度与引用环受限） */
export function exampleFor(
  doc: OpenApiDocument,
  schema: Schema | undefined,
  depth = 0,
  stack: readonly string[] = [],
): unknown {
  if (!schema || depth > 6) return null
  if (schema.example !== undefined) return schema.example
  if (schema.$ref) {
    const name = refName(schema.$ref)
    if (stack.includes(name)) return {}
    return exampleFor(doc, doc.components?.schemas?.[name], depth + 1, [...stack, name])
  }
  const union = schema.oneOf ?? schema.anyOf
  if (union) {
    const first = union.find((x) => x.type !== 'null')
    return first ? exampleFor(doc, first, depth + 1, stack) : null
  }
  if (schema.allOf) {
    return schema.allOf.reduce<Record<string, unknown>>((acc, part) => {
      const v = exampleFor(doc, part, depth + 1, stack)
      return v && typeof v === 'object' && !Array.isArray(v) ? { ...acc, ...v } : acc
    }, {})
  }
  if (schema.enum) return schema.enum.find((v) => v !== null) ?? null
  if (schema.default !== undefined) return schema.default
  const { type } = nonNullType(schema.type)
  if (type === 'object' || schema.properties) {
    const out: Record<string, unknown> = {}
    for (const [k, v] of Object.entries(schema.properties ?? {})) {
      out[k] = exampleFor(doc, v, depth + 1, stack)
    }
    return out
  }
  if (type === 'array') {
    const item = exampleFor(doc, schema.items, depth + 1, stack)
    return item === null ? [] : [item]
  }
  switch (type) {
    case 'string':
      return STRING_FORMATS[schema.format ?? ''] ?? 'string'
    case 'integer':
      return 0
    case 'number':
      return 0
    case 'boolean':
      return false
    default:
      return null
  }
}

/** 对象 Schema 的一级字段表 */
export function schemaFields(doc: OpenApiDocument, schema: Schema | undefined): SchemaField[] {
  const s = resolveSchema(doc, schema)
  if (s.allOf) return s.allOf.flatMap((part) => schemaFields(doc, part))
  const required = new Set(s.required ?? [])
  return Object.entries(s.properties ?? {}).map(([name, prop]) => ({
    name,
    type: typeName(prop),
    required: required.has(name),
    description: (prop.description ?? resolveSchema(doc, prop).description ?? '').trim(),
  }))
}

/** 展平为操作列表（按 tag → path → 方法顺序排序） */
export function listOperations(doc: OpenApiDocument): ApiOperation[] {
  const ops: ApiOperation[] = []
  for (const [path, item] of Object.entries(doc.paths ?? {})) {
    for (const method of HTTP_METHODS) {
      const op = item?.[method]
      if (!op) continue
      const bodyEntry = Object.entries(op.requestBody?.content ?? {})[0]
      ops.push({
        key: `${method} ${path}`,
        method,
        path,
        tag: op.tags?.[0] ?? 'default',
        operationId: op.operationId ?? '',
        summary: (op.summary ?? '').trim(),
        description: (op.description ?? '').trim(),
        permission: op['x-permission'] ?? '',
        secured: Array.isArray(op.security) && op.security.length > 0,
        parameters: op.parameters ?? [],
        requestBody: bodyEntry
          ? {
              contentType: bodyEntry[0],
              required: !!op.requestBody?.required,
              schema: bodyEntry[1].schema,
            }
          : undefined,
        responses: Object.entries(op.responses ?? {})
          .sort(([a], [b]) => a.localeCompare(b))
          .map(([status, r]) => {
            const media = Object.entries(r.content ?? {})[0]
            return {
              status,
              description: r.description ?? '',
              contentType: media?.[0],
              schema: media?.[1].schema,
            }
          }),
      })
    }
  }
  return ops.sort(
    (a, b) =>
      a.tag.localeCompare(b.tag) ||
      a.path.localeCompare(b.path) ||
      HTTP_METHODS.indexOf(a.method) - HTTP_METHODS.indexOf(b.method),
  )
}

/** 按 tag 分组（保持 listOperations 的顺序） */
export function groupByTag(
  ops: readonly ApiOperation[],
): Array<{ tag: string; operations: ApiOperation[] }> {
  const groups = new Map<string, ApiOperation[]>()
  for (const op of ops) {
    const list = groups.get(op.tag)
    if (list) list.push(op)
    else groups.set(op.tag, [op])
  }
  return [...groups.entries()].map(([tag, operations]) => ({ tag, operations }))
}

/**
 * 多关键字搜索（空格分隔，全部命中才算）：匹配路径、摘要、说明、operationId、权限、tag；
 * 关键字恰为 HTTP 方法名时只按方法过滤（`post users`）。
 */
export function filterOperations(ops: readonly ApiOperation[], query: string): ApiOperation[] {
  const terms = query.trim().toLowerCase().split(/\s+/).filter(Boolean)
  if (terms.length === 0) return [...ops]
  return ops.filter((op) => {
    const hay = [op.path, op.summary, op.description, op.operationId, op.permission, op.tag]
      .join(' ')
      .toLowerCase()
    return terms.every((t) =>
      (HTTP_METHODS as readonly string[]).includes(t) ? op.method === t : hay.includes(t),
    )
  })
}

/** 路径参数替换为示例值：`/api/users/{id}` → `/api/users/1` */
export function samplePath(path: string): string {
  return path.replace(/\{[^}]+\}/g, '1')
}

function shellQuote(s: string): string {
  return `'${s.replace(/'/g, `'\\''`)}'`
}

/** 请求体 JSON 示例（非 JSON 请求体返回 undefined） */
export function requestExample(doc: OpenApiDocument, op: ApiOperation): unknown {
  if (!op.requestBody || op.requestBody.contentType !== 'application/json') return undefined
  return exampleFor(doc, op.requestBody.schema)
}

/** 部署主机上的等价命令（fpctl 现签令牌，免登录；脚本与 AI 直接可用） */
export function fpctlCommand(op: ApiOperation, body?: unknown): string {
  let cmd = `fpctl api ${op.method.toUpperCase()} ${shellQuote(samplePath(op.path))}`
  if (body !== undefined) cmd += ` -d ${shellQuote(JSON.stringify(body))}`
  return `${cmd} --json`
}

/** 远程调用示例（需自备令牌） */
export function curlCommand(op: ApiOperation, origin: string, body?: unknown): string {
  const parts = [`curl -X ${op.method.toUpperCase()} ${shellQuote(origin + samplePath(op.path))}`]
  // 双引号：让 shell 展开 $TOKEN
  if (op.secured) parts.push('-H "Authorization: Bearer $TOKEN"')
  if (body !== undefined) {
    parts.push(`-H 'Content-Type: application/json'`)
    parts.push(`-d ${shellQuote(JSON.stringify(body))}`)
  }
  return parts.join(' \\\n  ')
}
