import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { reactive } from 'vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'
import type { OpenApiDocument } from '@/utils/openapi'

const doc: OpenApiDocument = {
  info: { version: '1.0.0' },
  paths: {
    '/api/auth/login': {
      post: {
        summary: 'Login',
        tags: ['auth'],
        requestBody: {
          content: {
            'application/json': {
              schema: {
                type: 'object',
                required: ['username'],
                properties: { username: { type: 'string' }, password: { type: 'string' } },
              },
            },
          },
        },
        responses: { '200': { description: 'ok' } },
      },
    },
    '/api/memos': {
      get: {
        summary: 'List memos',
        tags: ['memo'],
        security: [{ BearerAuth: [] }],
        'x-permission': 'memo:read',
        responses: { '200': { description: 'ok' } },
      },
      post: {
        summary: 'Create memo',
        tags: ['memo'],
        security: [{ BearerAuth: [] }],
        'x-permission': 'memo:write',
        responses: { '200': { description: 'ok' } },
      },
    },
  },
}

const getOpenApiDocument = vi.fn()
vi.mock('@/api/openapiDoc', () => ({ getOpenApiDocument: () => getOpenApiDocument() }))

// 路由替身：query 为响应式对象，replace 直接写回——与真实 router.replace 的可观察效果一致
const route = reactive<{ query: Record<string, string | undefined> }>({ query: {} })
const replace = vi.fn(async ({ query }: { query: Record<string, string | undefined> }) => {
  route.query = Object.fromEntries(Object.entries(query).filter(([, v]) => v !== undefined))
})
vi.mock('vue-router', () => ({ useRoute: () => route, useRouter: () => ({ replace }) }))

const ApiDocsView = (await import('@/views/ApiDocsView.vue')).default

const messages = {
  apiDocs: {
    title: 'API',
    downloadSpec: 'Download',
    empty: 'No endpoints',
    modules: 'Modules',
    allModules: 'All',
    search: 'Search',
    searchPlaceholder: 'Search…',
    allMethods: 'All methods',
    showing: '{shown} / {total}',
    noMatch: 'No match',
    noMatchHint: 'Try another keyword',
    endpoints: 'Endpoints',
    publicEndpoints: 'Public',
  },
  pageDesc: { apiDocs: 'desc' },
}

function mountView() {
  return mount(ApiDocsView, {
    global: {
      plugins: [createTestI18n(messages), openVuePlugin, ...services],
    },
  })
}

describe('ApiDocsView', () => {
  beforeEach(() => {
    route.query = {}
    replace.mockClear()
    getOpenApiDocument.mockReset()
    getOpenApiDocument.mockResolvedValue({ data: doc })
  })

  it('lists every operation grouped by tag with counts', async () => {
    const w = mountView()
    await flushPromises()
    expect(getOpenApiDocument).toHaveBeenCalledTimes(1)
    expect(w.findAll('.api-op')).toHaveLength(3)
    expect(w.findAll('.api-docs__group-title').map((h) => h.text())).toEqual(['auth 1', 'memo 2'])
    expect(w.find('.api-docs__stats').text()).toContain('3 / 3')
    expect(w.find('.api-docs__stats').text()).toContain('v1.0.0')
  })

  it('filters by the URL-synced search and tag', async () => {
    const w = mountView()
    await flushPromises()

    await w.find('.api-docs__search input').setValue('memo:write')
    await flushPromises()
    expect(replace).toHaveBeenLastCalledWith({ query: { q: 'memo:write' } })
    expect(w.findAll('.api-op')).toHaveLength(1)

    route.query = {}
    await flushPromises()
    const memoTag = w.findAll('.api-docs__tag').find((b) => b.text().startsWith('memo'))!
    await memoTag.trigger('click')
    await flushPromises()
    expect(route.query.tag).toBe('memo')
    expect(w.findAll('.api-op')).toHaveLength(2)
    expect(memoTag.attributes('aria-pressed')).toBe('true')
  })

  it('shows a no-match hint when nothing matches', async () => {
    route.query = { q: 'definitely-absent' }
    const w = mountView()
    await flushPromises()
    expect(w.findAll('.api-op')).toHaveLength(0)
    expect(w.text()).toContain('No match')
  })

  it('expands an operation via ?op= and renders runnable commands', async () => {
    const w = mountView()
    await flushPromises()
    expect(w.find('.api-op.is-open').exists()).toBe(false)

    await w.findAll('.api-op button')[0]!.trigger('click')
    await flushPromises()
    expect(route.query.op).toBe('post /api/auth/login')
    const open = w.find('.api-op.is-open')
    expect(open.exists()).toBe(true)
    expect(open.text()).toContain(`fpctl api POST '/api/auth/login'`)
    expect(open.text()).toContain('username')
  })

  it('surfaces load errors with a retry', async () => {
    getOpenApiDocument.mockRejectedValueOnce(new Error('boom'))
    const w = mountView()
    await flushPromises()
    expect(w.text()).toContain('boom')
    expect(w.findAll('.api-op')).toHaveLength(0)
  })
})
