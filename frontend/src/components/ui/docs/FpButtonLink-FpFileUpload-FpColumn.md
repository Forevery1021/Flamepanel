# FpButtonLink / FpFileUpload / FpColumn

## FpButtonLink

文本链接按钮（原 `Button text`）。

- 底层：`openvue/button`

| 名 | 类型 | 默认 | 说明 |
|----|------|------|------|
| `label` | `string` | `''` | 文案 |
| `text` | `boolean` | `true` | 文本样式 |
| `size` | `'small' \| 'large'` | `'small'` | 尺寸 |

```vue
<FpButtonLink @click="edit(row)">{{ t('common.edit') }}</FpButtonLink>
```

## FpFileUpload

文件上传。

- 底层：`openvue/fileupload`；无额外 props（透传 `multiple`/`accept`/`auto`/`customUpload` 等）。

```vue
<FpFileUpload :custom-upload="true" @select="onSelect" />
```

## FpColumn

表格列定义，透传至 OpenVue `Column`。**实现为透明再导出（`export default Column`），不是包装组件**。

- 底层：`openvue/column`；props 全部透传（`field`/`header`/`style`/`frozen`/`align-frozen`/`sortable` 等）。
- ⚠️ **机制约束（勿改回包装写法）**：DataTable 通过 `child.type.name === 'Column'`（HelperSet）收集列，并在渲染期通过 `vnode.type.extends.props[prop]`（getVNodeProp）读取列 prop 元数据。任何中间包装组件都会导致 ① name 不匹配时**列静默丢失（表格空白且无报错）**；② name 对齐后 `extends.props` 不存在直接 TypeError。因此封装层只能保持 VNode type 即 Column 本身；如需统一样式，请在 FpTable 的 `:deep()` 或 pt 透传层实现。
- 回归保护：`src/components/ui/__tests__/FpTable.spec.ts`（同一性断言 + 裸 Column / FpColumn 渲染对比）。

```vue
<FpColumn field="name" :header="t('docker.name')" style="min-width: 140px">
  <template #body="{ data }">
    <span>{{ data.name }}</span>
  </template>
</FpColumn>
```
