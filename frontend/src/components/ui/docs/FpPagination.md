# FpPagination

统一分页条。

- 底层：`openvue/paginator`
- 位置：`@/components/ui/FpPagination.vue`

## 用途

列表分页。`total <= pageSize` 时自动隐藏。通常与 `FpTable` / `FpStatePanel` 配合。

## Props

| 名 | 类型 | 默认 | 说明 |
|----|------|------|------|
| `total` | `number` | —（必填） | 总记录数 |
| `pageSize` | `number` | `10` | 每页行数 |
| `first` | `number` | `0` | 当前首行索引（受控） |
| `rowsPerPageOptions` | `number[]` | `[20,50,100]` | 每页可选行数 |
| `template` | `string` | 默认模板 | 分页条模板 |
| `currentPageReportTemplate` | `string` | — | 报告模板 |

## Events

| 名 | 载荷 | 说明 |
|----|------|------|
| `update:first` | `(value: number)` | 翻页时发出新首行索引 |

## 示例

```vue
<FpPagination
  v-if="total > pageSize"
  :first="(currentPage - 1) * pageSize"
  :rows="pageSize"
  :total="total"
  :rows-per-page-options="[20, 50, 100]"
  @update:first="(f) => goPage(f)"
/>
```
