# FpStatePanel

统一 loading / error(可重试) / empty 三态容器。

- 位置：`@/components/ui/FpStatePanel.vue`

## 用途

列表页三态规范化：加载中骨架、错误可重试、空态。状态机：`loading → success | error`；空列表 = `success + empty`（非 error）。

## Props

| 名 | 类型 | 默认 | 说明 |
|----|------|------|------|
| `loading` | `boolean` | `false` | 加载中 |
| `error` | `string \| null` | `null` | 错误信息（非空即错误态） |
| `empty` | `boolean` | `false` | 空态判定（配合 loading/error 后判断） |
| `hideEmpty` | `boolean` | `false` | 空态时隐藏 |
| `title` | `string` | `''` | 错误态标题 |
| `description` | `string` | `''` | 错误态描述（默认取 error） |
| `retryable` | `boolean` | `false` | 是否显示重试按钮 |
| `retryText` | `string` | `''` | 重试按钮文案 |
| `loadingText` | `string` | `''` | 加载文案 |
| `emptyIcon` | `string` | `'oi oi-inbox'` | 空态图标 |
| `emptyTitle` | `string` | `''` | 空态标题 |
| `emptyDesc` | `string` | `''` | 空态描述 |

## Events

| 名 | 载荷 | 说明 |
|----|------|------|
| `retry` | — | 点击重试按钮 |

## 示例

```vue
<FpStatePanel
  :loading="loading"
  :error="usersError"
  :empty="!total && !loading && !usersError"
  retryable
  :empty-title="t('common.noData')"
  @retry="fetch"
>
  <FpTable :rows="users" :loading="loading">...</FpTable>
  <FpPagination .../>
</FpStatePanel>
```
