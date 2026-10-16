import { readFile, writeFile } from 'node:fs/promises'
import { pathToFileURL } from 'node:url'
import openapiTS, { astToString } from 'openapi-typescript'

const methods = ['get', 'put', 'post', 'delete', 'options', 'head', 'patch', 'trace']

/** Keep the server snapshot intact; method + exact path is a globally unique SDK operation key. */
export function withStableOperationIds(schema) {
  const normalized = structuredClone(schema)
  for (const [path, item] of Object.entries(normalized.paths ?? {})) {
    if (!item || typeof item !== 'object') continue
    for (const method of methods) {
      const operation = item[method]
      if (operation && typeof operation === 'object') {
        operation.operationId = `${method.toUpperCase()} ${path}`
      }
    }
  }
  return normalized
}

async function main() {
  const input = new URL('../openapi.json', import.meta.url)
  const output = new URL('../src/api/generated/openapi.ts', import.meta.url)
  const schema = JSON.parse(await readFile(input, 'utf8'))
  const ast = await openapiTS(withStableOperationIds(schema))
  await writeFile(output, astToString(ast))
  console.log('Generated API types with stable HTTP-method/path operation IDs.')
}

if (process.argv[1] && pathToFileURL(process.argv[1]).href === import.meta.url) {
  await main()
}
