import { describe, expect, it } from 'vitest'
import openapiTS, { astToString } from 'openapi-typescript'
import ts from 'typescript'
import { withStableOperationIds } from './generate-api.mjs'

const response = { 200: { description: 'OK' } }
const schema = {
  openapi: '3.1.0',
  info: { title: 'Fixture', version: '1' },
  paths: {
    '/api/nodes': {
      get: { operationId: 'list', responses: response },
      post: { operationId: 'create', responses: response },
    },
    '/api/websites': { get: { operationId: 'list', responses: response } },
  },
}

describe('API generator operation identifiers', () => {
  it('separates repeated handler names and HTTP methods without mutating the snapshot', () => {
    const original = structuredClone(schema)
    const normalized = withStableOperationIds(schema)
    expect(normalized.paths['/api/nodes'].get.operationId).toBe('GET /api/nodes')
    expect(normalized.paths['/api/nodes'].post.operationId).toBe('POST /api/nodes')
    expect(normalized.paths['/api/websites'].get.operationId).toBe('GET /api/websites')
    expect(schema).toEqual(original)
    expect(normalized.paths['/api/nodes'].get.responses).toEqual(response)
  })

  it('does not conflate punctuation, braces, or path case', () => {
    const paths = ['/api/a-b', '/api/a_b', '/api/{id}', '/api/id', '/api/ID']
    const normalized = withStableOperationIds({
      paths: Object.fromEntries(paths.map((path) => [path, { get: { operationId: 'get' } }])),
    })
    const ids = Object.values(normalized.paths).map((item) => item.get.operationId)
    expect(new Set(ids).size).toBe(paths.length)
  })

  it('preserves path-level metadata and supports a schema with no paths', () => {
    const input = {
      paths: { '/api/x': { parameters: [{ name: 'id', in: 'path' }], summary: 'x' } },
    }
    expect(withStableOperationIds(input)).toEqual(input)
    expect(withStableOperationIds({ components: {} })).toEqual({ components: {} })
  })

  it('emits valid quoted TypeScript operation keys with the project compiler', async () => {
    const output = astToString(await openapiTS(withStableOperationIds(schema)))
    expect(output).toContain('"GET /api/nodes"')
    expect(output).toContain('"GET /api/websites"')
    const source = ts.createSourceFile(
      'fixture.ts',
      output,
      ts.ScriptTarget.Latest,
      true,
      ts.ScriptKind.TS,
    )
    expect(source.parseDiagnostics).toEqual([])
  })
})
