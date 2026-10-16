---
name: flamepanel-frontend-view
description: 在 FlamePanel 新增或修改前端视图、组件、composable、api 模块、i18n、路由时使用。当任务涉及 Vue 视图、vue-query、Fp* 组件、v-permission、三语言 key、vitest spec、视图四层拆分时使用。给出与当前 frontend/src 一致的六步链路与模板。
---

# FlamePanel · 前端视图

> 参照：`views/WafView.vue`（最新、≤300 行）+ `composables/useWebsiteList.ts` + `components/website/*` + `views/__tests__/MemoList.spec.ts`。

## 1. 六步链路

```text
① src/api/foo.ts             axios 封装，类型只从 '@/api/generated' 取
② src/api/queryKeys.ts       追加 foo key 工厂
③ src/composables/useFoo*.ts vue-query 数据层（列表 / 增删改 / 失效）
④ src/components/foo/*.vue   表单字段、弹窗、列表面板（视图 ≤ 250 行，超了拆到这里）
   src/views/FooView.vue     只装配 Fp* 与 components/foo
⑤ src/locales/{zh-CN,en-US,ja-JP}/<模块>.ts  三语言同 key；新域则三处 index.ts 同时 import
⑥ src/router/index.ts       meta.perm + i18n title；spec 覆盖
```

## 2. 模板

```ts
// ① api/foo.ts
import api from './client'
import type { Foo, CreateFooRequest } from '@/api/generated'
export const listFoos = (page = 1, pageSize = 20) => api.get<{ data: Foo[]; total: number }>('/foos', { params: { page, page_size: pageSize } })
export const createFoo = (input: CreateFooRequest) => api.post<Foo>('/foos', input)
export const deleteFoo = (id: number) => api.delete<void>(`/foos/${id}`)

// ② api/queryKeys.ts
foo: { all: ['foo'] as const, list: (page: number) => ['foo', 'list', page] as const },

// ③ composables/useFooList.ts
import { useQuery, useMutation, useQueryClient } from '@tanstack/vue-query'
export function useFooList(page: Ref<number>) {
  const qc = useQueryClient()
  const query = useQuery({
    queryKey: computed(() => queryKeys.foo.list(page.value)),
    queryFn: () => listFoos(page.value).then(r => r.data),
    placeholderData: keepPreviousData,
  })
  const remove = useMutation({ mutationFn: deleteFoo, onSuccess: () => qc.invalidateQueries({ queryKey: queryKeys.foo.all }) })
  return { ...query, remove }
}
```

```vue
<!-- ④ views/FooView.vue -->
<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useFooList } from '@/composables/useFooList'
import { useFpToast } from '@/components/ui/FpToast'
const { t } = useI18n()
const page = ref(1)
const { data, isLoading, error, refetch, remove } = useFooList(page)
const toast = useFpToast()
async function onDelete(id: number) {
  await remove.mutateAsync(id); toast.success(t('common.deleted'))
}
</script>
<template>
  <LayoutContent :title="t('foo.title')" reload @reload="refetch">
    <template #actions>
      <FpButton v-permission="{ perm: 'foo:create', mode: 'view' }" variant="primary" icon="oi oi-plus">{{ t('foo.create') }}</FpButton>
    </template>
    <FpStatePanel :loading="isLoading" :error="error" :empty="!data?.data.length">
      <FpTable :rows="data?.data ?? []">
        <FpColumn field="name" :header="t('foo.name')" />
        <FpColumn :header="t('common.actions')" style="width: 120px">
          <template #body="{ data: row }">
            <FpButton v-permission="{ perm: 'foo:delete', mode: 'view' }" variant="danger" size="sm" @click="onDelete(row.id)">{{ t('common.delete') }}</FpButton>
          </template>
        </FpColumn>
      </FpTable>
      <FpPagination v-model="page" :total="data?.total ?? 0" />
    </FpStatePanel>
  </LayoutContent>
</template>
```

```ts
// ⑤ locales/zh-CN/foo.ts（en-US / ja-JP 同结构）
export default { foo: { title: 'Foo 管理', create: '新建', name: '名称' } }
// locales/{lang}/index.ts：import foo from './foo' 并展开——三个 index.ts 顺序一致（locale:check 校验）

// ⑥ router/index.ts
{ path: '/foos', name: 'foos', component: () => import('@/views/FooView.vue'),
  meta: { title: 'nav.foos', icon: 'oi-box', group: 'ops' as MenuGroup, keepAlive: true, weight: 35, keywords: ['foo'], perm: 'foo:read' } },
// nav.foos 也要三语言（shell.ts）
```

## 3. 硬规范（eslint / CI 会拦）

| 规范 | 拦截方式 |
|---|---|
| views 只 import `@/components/ui`（Fp*）与 `@/components/<域>`；禁止 `openvue/*` 深路径 | eslint `no-restricted-imports` |
| 类型来自 `@/api/generated`，不手写接口类型；不 `as any` | typecheck + review |
| 文案全部 `t('…')`，三语言 key 集合相等、占位符一致 | `npm run locale:check` |
| 权限：`v-permission="{ perm: 'res:action', mode: 'view' }"`，字面与后端 `permissions.rs` 一致 | review |
| 三态用 `FpStatePanel`，列表用 `FpTable`，不手写 `v-if="loading"` 散逻辑 | review |
| 轮询 / WS / ECharts 用 `usePolling` / `useSharedWebSocket` / `useECharts`（keep-alive 安全） | review |
| 视图 ≤ 250 行；组件 ≤ 300 行 | review（T2 拆分纪律） |
| 覆盖率地板 60/60/68/62、首屏 200 KB | `test:coverage` / `bundle:check` |

## 4. 测试模板（views/__tests__/FooView.spec.ts）

```ts
import { describe, it, expect, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import FooView from '@/views/FooView.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

vi.mock('@/api/foo', () => ({
  listFoos: vi.fn().mockResolvedValue({ data: { data: [{ id: 1, name: 'a', enabled: true, created_at: '' }], total: 1 } }),
  deleteFoo: vi.fn().mockResolvedValue({ data: undefined }),
}))

function mountView(perms = ['foo:read', 'foo:create', 'foo:delete']) {
  return mount(FooView, { global: { plugins: [createTestI18n(), openVuePlugin, ...services({ permissions: perms })] } })
}

describe('FooView', () => {
  it('renders rows', async () => { const w = mountView(); await flushPromises(); expect(w.text()).toContain('a') })
  it('hides create button without permission', async () => {
    const w = mountView(['foo:read']); await flushPromises()
    expect(w.find('[data-test="create"]').exists()).toBe(false)     // 给按钮加 data-test
  })
  it('shows empty state', async () => { /* mock 返回空数组 → 断言 FpStatePanel 空态文案 */ })
})
```

> `services()` helper 的真实签名以 `components/ui/__tests__/helpers.ts` 为准——**先 grep 再用**。

## 5. 命令

```bash
cd frontend
npm run generate:api                  # 后端 DTO 变了先重新生成
npx vitest run src/views/__tests__/FooView.spec.ts
npm run typecheck && npm run lint && npm run test:unit && npm run build
npm run test:coverage && npm run bundle:check && npm run locale:check
```

## 6. 自查

- [ ] 六步齐；三语言 key 数一致；`locale:check` 绿
- [ ] 视图 ≤ 250 行；无 `openvue/*` import；无中文字面量
- [ ] spec ≥ 3（渲染 / 空态 / 无权限），coverage 地板未跌
- [ ] 若新增路由：`nav.*` key 三语言；命令面板 `keywords` 已填

