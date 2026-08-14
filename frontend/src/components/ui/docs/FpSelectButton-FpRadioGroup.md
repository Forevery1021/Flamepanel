# FpSelectButton / FpRadioGroup / FpRadioOption

统一单选按钮组。

## FpSelectButton

- 底层：`openvue/selectbutton`

### v-model

`v-model` 绑定 `string` 选中值。

```vue
<FpSelectButton v-model="density" :options="densityOptions" option-label="label" option-value="value" />
```

## FpRadioGroup / FpRadioOption

- 底层：`openvue/radiobuttongroup` + `openvue/radiobutton`

### FpRadioGroup Props

| 名 | 类型 | 默认 | 说明 |
|----|------|------|------|
| `options` | `Record<string, unknown>[]` | `[]` | 选项数组 |
| `optionLabel` | `string` | `'label'` | 选项展示字段 |
| `optionValue` | `string` | `'value'` | 选项值字段 |

### v-model

`v-model` 绑定 `string` 选中值。

### 示例

```vue
<FpRadioGroup v-model="role" :options="roleOptions" option-label="label" option-value="value" />
<!-- 或自定义插槽 -->
<FpRadioGroup v-model="mode">
  <FpRadioOption value="dark" :input-id="'mode-dark'" />
</FpRadioGroup>
```
