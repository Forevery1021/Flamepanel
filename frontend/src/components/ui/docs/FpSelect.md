# FpSelect

统一下拉选择。

- 底层：`openvue/select`
- 位置：`@/components/ui/FpSelect.vue`

## 用途

单选 / 多选 / 可过滤 / 可清除的下拉。

## Props

| 名            | 类型        | 默认      | 说明                                                                                 |
| ------------- | ----------- | --------- | ------------------------------------------------------------------------------------ |
| `label`       | `string`    | `''`      | 浮动标签                                                                             |
| `error`       | `string`    | `''`      | 错误文案                                                                             |
| `invalid`     | `boolean`   | `false`   | 手动非法态                                                                           |
| `showClear`   | `boolean`   | `false`   | 显示清除按钮                                                                         |
| `filter`      | `boolean`   | `false`   | 允许过滤搜索                                                                         |
| `multiple`    | `boolean`   | `false`   | 已废弃（底层单选组件会忽略），多选请用 `FpMultiSelect`                               |
| `options`     | `unknown[]` | `[]`      | 选项数组                                                                             |
| `optionLabel` | `string`    | `'label'` | 选项展示字段                                                                         |
| `optionValue` | `string`    | `'value'` | 选项值字段                                                                           |
| `placeholder` | `string`    | `''`      | 占位文本                                                                             |
| `disabled`    | `boolean`   | `false`   | 禁用                                                                                 |
| `inputId`     | `string`    | 自动生成  | 内部可聚焦控件（`role=combobox`）的 id，供外部 `<label for>` 关联（如 `SettingRow`） |

其余属性经 `$attrs` 透传。

## v-model

`v-model` 绑定 `string | number`。

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

## FpMultiSelect

多选一律用 `FpMultiSelect`（封装 `openvue/multiselect`）；`v-model` 恒为数组（清空得到 `[]` 而非 `null`）。支持 `label` / `error` / `invalid` / `filter` / `inputId`，选项相关属性经 `$attrs` 透传。

```vue
<FpMultiSelect
  v-model="languages"
  :options="langOptions"
  option-label="label"
  option-value="value"
  filter
/>
```

> 注意：`FpSelect` 底层是单选组件，`multiple` 会被静默忽略——需要多选时必须用 `FpMultiSelect`。
