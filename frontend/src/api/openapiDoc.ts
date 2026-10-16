import api from './client'
import type { OpenApiDocument } from '@/utils/openapi'

/** 本版本内置的 OpenAPI 文档（`GET /api/openapi.json`，免登录；与 `fpctl openapi` 同源） */
export function getOpenApiDocument() {
  return api.get<OpenApiDocument>('/openapi.json')
}
