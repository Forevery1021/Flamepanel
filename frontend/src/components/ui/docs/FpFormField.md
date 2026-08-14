# FpFormField

统一表单字段容器：label + slot + error/hint。

- 位置：`@/components/ui/FpFormField.vue`

## 用途

表单中非输入类字段（或需要自定义输入布局）时，用其承载 label 与错误/提示。

## Props

| 名 | 类型 | 默认 | 说明 |
|----|------|------|------|
| `label` | `string` | `''` | 字段标签 |
| `error` | `string` | `''` | 错误文案 |
| `hint` | `string` | `''` | 辅助提示 |

## Slots

| 名 | 说明 |
|----|------|
| `default` | 字段内容 |

## 示例

```vue
<FpFormField :label="t('settings.density')" :error="densityError">
  <FpSelectButton v-model="density" :options="densityOptions" />
</FpFormField>
```
