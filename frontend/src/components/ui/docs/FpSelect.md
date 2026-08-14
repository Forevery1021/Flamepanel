# FpSelect

统一下拉选择。

- 底层：`openvue/select`
- 位置：`@/components/ui/FpSelect.vue`

## 用途

单选 / 多选 / 可过滤 / 可清除的下拉。

## Props

| 名 | 类型 | 默认 | 说明 |
|----|------|------|------|
| `label` | `string` | `''` | 浮动标签 |
| `error` | `string` | `''` | 错误文案 |
| `invalid` | `boolean` | `false` | 手动非法态 |
| `showClear` | `boolean` | `false` | 显示清除按钮 |
| `filter` | `boolean` | `false` | 允许过滤搜索 |
| `multiple` | `boolean` | `false` | 多选 |
| `options` | `unknown[]` | `[]` | 选项数组 |
| `optionLabel` | `string` | `'label'` | 选项展示字段 |
| `optionValue` | `string` | `'value'` | 选项值字段 |
| `placeholder` | `string` | `''` | 占位文本 |
| `disabled` | `boolean` | `false` | 禁用 |

其余属性经 `$attrs` 透传。

## v-model

`v-model` 绑定 `string | number | string[]`（单选为标量，`multiple` 为数组）。

## 示例

```vue
<FpSelect
  v-model="actionFilter"
  :options="actionOptions"
  option-label="label"
  option-value="value"
  show-clear
  @update:model-value="fetch"
/>
```
