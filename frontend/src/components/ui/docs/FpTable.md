# FpTable

统一数据表格：加载骨架、空态、分页、虚拟滚动、客户端排序。

- 底层：`openvue/datatable`（配 `FpColumn` 定义列）
- 位置：`@/components/ui/FpTable.vue`

## 用途

所有资源列表页（Docker / Users / Logs / 文件等）的统一表格。大列表开启 `virtual` 虚拟滚动。

## Props

| 名 | 类型 | 默认 | 说明 |
|----|------|------|------|
| `rows` | `unknown[]` | —（必填） | 表格数据 |
| `loading` | `boolean` | `false` | 加载态（骨架/遮罩） |
| `stripedRows` | `boolean` | `false` | 斑马纹 |
| `virtual` | `boolean` | `false` | 虚拟滚动（大列表；启用后自动 scrollable，自动禁用分页） |
| `virtualItemSize` | `number` | `44` | 虚拟行高（px） |
| `virtualScrollHeight` | `string` | `'520px'` | 虚拟滚动视口高度 |
| `scrollable` | `boolean` | `false` | 普通滚动模式（配 `scrollHeight`） |
| `scrollHeight` | `string` | `''` | 滚动视口高度 |
| `sortable` | `boolean` | `false` | 客户端单列排序 |
| `sortField` | `string \| null` | `null` | 当前排序列字段 |
| `sortOrder` | `1 \| -1 \| 0` | `0` | 排序方向 |
| `paginator` | `boolean` | `true` | 是否显示分页 |
| `rowsPerPage` | `number` | `10` | 每页行数 |
| `first` | `number` | `0` | 当前首行索引（受控分页） |
| `rowsPerPageOptions` | `number[]` | `[10,20,50,100]` | 每页可选行数 |
| `size` | `'small' \| 'large'` | `'small'` | 密度 |
| `paginatorTemplate` | `string` | 默认模板 | 分页条模板 |
| `currentPageReportTemplate` | `string` | `'{first}-{last} / {totalRecords}'` | 分页报告模板 |
| `emptyText` | `string` | `''` | 空态文案 |
| `emptyDesc` | `string` | `''` | 空态描述 |
| `loadingText` | `string` | `''` | 加载态文案 |

## Events

| 名 | 载荷 | 说明 |
|----|------|------|
| `update:first` | `(value: number)` | 分页翻页时发出新的首行索引（配合 `FpPagination` 使用） |

## Slots

| 名 | 说明 |
|----|------|
| `empty` | 空态自定义内容 |
| `loading` | 加载态自定义内容 |

配合 `FpColumn` 使用：

```vue
<FpTable :rows="users" :loading="loading" :paginator="false">
  <FpColumn field="username" :header="t('user.username')" />
  <FpColumn field="role" :header="t('user.role')">
    <template #body="{ data }">
      <FpTag :value="data.role" />
    </template>
  </FpColumn>
</FpTable>
```

## 说明

- 大列表（1000+ 行）务必开启 `virtual` 与 `virtualScrollHeight`，避免整页卡顿。
- 虚拟滚动会自动关闭分页，分页下沉到后端 `list_page` + `count`。
