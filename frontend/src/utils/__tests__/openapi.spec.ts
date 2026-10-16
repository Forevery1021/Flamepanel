import { describe, expect, it } from 'vitest'
import {
  curlCommand,
  exampleFor,
  filterOperations,
  fpctlCommand,
  groupByTag,
  listOperations,
  requestExample,
  samplePath,
  schemaFields,
  typeName,
  type OpenApiDocument,
} from '@/utils/openapi'

/** 贴近 utoipa 5 实际输出（OpenAPI 3.1：可空为 type 数组） */
const doc: OpenApiDocument = {
  info: { version: '1.0.0' },
  paths: {
    '/api/users/{id}': {
      delete: {
        operationId: 'delete_user',
        summary: '删除用户',
        tags: ['users'],
        security: [{ BearerAuth: [] }],
        'x-permission': 'user:delete',
        parameters: [
          { name: 'id', in: 'path', required: true, schema: { type: 'integer', format: 'int64' } },
        ],
        responses: { '200': { description: 'ok' }, '404': { description: '不存在' } },
      },
      put: {
        operationId: 'update_user',
        summary: '更新用户',
        tags: ['users'],
        security: [{ BearerAuth: [] }],
        requestBody: {
          required: true,
          content: { 'application/json': { schema: { $ref: '#/components/schemas/UpdateUser' } } },
        },
        responses: {
          '200': {
            description: 'ok',
            content: { 'application/json': { schema: { $ref: '#/components/schemas/User' } } },
          },
        },
      },
    },
    '/api/auth/login': {
      post: {
        operationId: 'login',
        summary: '登录',
        tags: ['auth'],
        requestBody: {
          content: { 'application/json': { schema: { $ref: '#/components/schemas/Login' } } },
        },
        responses: { '200': { description: 'ok' } },
      },
    },
    '/api/users': {
      get: {
        operationId: 'list_users',
        summary: '用户列表',
        tags: ['users'],
        security: [{ BearerAuth: [] }],
        responses: {},
      },
    },
  },
  components: {
    schemas: {
      Login: {
        type: 'object',
        required: ['username', 'password'],
        properties: {
          username: { type: 'string', description: '用户名' },
          password: { type: 'string' },
        },
      },
      UpdateUser: {
        type: 'object',
        properties: {
          role: { type: 'string', enum: ['admin', 'viewer'] },
          email: { type: ['string', 'null'] },
          tags: { type: 'array', items: { type: 'string' } },
        },
      },
      User: {
        type: 'object',
        required: ['id'],
        properties: {
          id: { type: 'integer', format: 'int64' },
          created_at: { type: 'string', format: 'date-time' },
          manager: { oneOf: [{ type: 'null' }, { $ref: '#/components/schemas/User' }] },
        },
      },
    },
  },
}

describe('listOperations', () => {
  it('flattens, sorts by tag → path → method and extracts metadata', () => {
    const ops = listOperations(doc)
    expect(ops.map((o) => o.key)).toEqual([
      'post /api/auth/login',
      'get /api/users',
      'put /api/users/{id}',
      'delete /api/users/{id}',
    ])
    const del = ops.find((o) => o.operationId === 'delete_user')!
    expect(del.permission).toBe('user:delete')
    expect(del.secured).toBe(true)
    expect(del.responses.map((r) => r.status)).toEqual(['200', '404'])
    const login = ops.find((o) => o.operationId === 'login')!
    expect(login.secured).toBe(false)
    expect(login.requestBody?.contentType).toBe('application/json')
    expect(groupByTag(ops).map((g) => [g.tag, g.operations.length])).toEqual([
      ['auth', 1],
      ['users', 3],
    ])
  })
})

describe('schema helpers', () => {
  it('renders type names', () => {
    expect(typeName({ $ref: '#/components/schemas/User' })).toBe('User')
    expect(typeName({ type: 'array', items: { $ref: '#/components/schemas/User' } })).toBe('User[]')
    expect(typeName({ type: ['string', 'null'] })).toBe('string?')
    expect(typeName({ type: 'string', format: 'date-time' })).toBe('string(date-time)')
    expect(typeName({ oneOf: [{ type: 'null' }, { $ref: '#/x/User' }] })).toBe('User?')
    expect(typeName({ type: 'object', additionalProperties: { type: 'integer' } })).toBe(
      'map<string, integer>',
    )
    expect(typeName({ type: 'string', enum: ['a', 'b'] })).toBe('"a" | "b"')
    expect(typeName(undefined)).toBe('any')
  })

  it('builds examples, surviving self-references', () => {
    const user = exampleFor(doc, { $ref: '#/components/schemas/User' }) as Record<string, unknown>
    expect(user.id).toBe(0)
    expect(user.created_at).toBe('2026-01-01T00:00:00Z')
    // 自引用：再次进入同一 $ref 时回落为空对象，而不是无限递归
    expect(user.manager).toEqual({})
    expect(exampleFor(doc, { $ref: '#/components/schemas/UpdateUser' })).toEqual({
      role: 'admin',
      email: 'string',
      tags: ['string'],
    })
  })

  it('lists first-level fields with required flags', () => {
    expect(schemaFields(doc, { $ref: '#/components/schemas/Login' })).toEqual([
      { name: 'username', type: 'string', required: true, description: '用户名' },
      { name: 'password', type: 'string', required: true, description: '' },
    ])
  })
})

describe('filterOperations', () => {
  const ops = listOperations(doc)
  it('matches all terms across path, summary and permission', () => {
    expect(filterOperations(ops, '').length).toBe(4)
    expect(filterOperations(ops, 'user:delete').map((o) => o.operationId)).toEqual(['delete_user'])
    expect(filterOperations(ops, '用户 更新').map((o) => o.operationId)).toEqual(['update_user'])
    expect(filterOperations(ops, 'nothing-here')).toEqual([])
  })
  it('treats a bare HTTP method as a method filter', () => {
    expect(filterOperations(ops, 'post').map((o) => o.operationId)).toEqual(['login'])
    expect(filterOperations(ops, 'GET users').map((o) => o.operationId)).toEqual(['list_users'])
  })
})

describe('command builders', () => {
  const ops = listOperations(doc)
  const put = ops.find((o) => o.operationId === 'update_user')!
  const login = ops.find((o) => o.operationId === 'login')!

  it('substitutes path parameters', () => {
    expect(samplePath('/api/a/{id}/b/{name}')).toBe('/api/a/1/b/1')
  })

  it('builds a runnable fpctl command with a shell-safe body', () => {
    const body = requestExample(doc, put)
    expect(fpctlCommand(put, body)).toBe(
      `fpctl api PUT '/api/users/1' -d '{"role":"admin","email":"string","tags":["string"]}' --json`,
    )
    expect(fpctlCommand(put, { note: "it's" })).toContain(`-d '{"note":"it'\\''s"}'`)
  })

  it('adds the bearer header (expandable $TOKEN) only for secured endpoints', () => {
    const secured = curlCommand(put, 'https://panel.example', requestExample(doc, put))
    expect(secured).toContain(`curl -X PUT 'https://panel.example/api/users/1'`)
    expect(secured).toContain('-H "Authorization: Bearer $TOKEN"')
    expect(secured).toContain(`-H 'Content-Type: application/json'`)
    const open = curlCommand(login, 'https://panel.example', requestExample(doc, login))
    expect(open).not.toContain('Authorization')
  })
})
