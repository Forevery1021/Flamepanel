# FpInput

统一文本输入（FloatLabel + 错误提示）。

- 底层：`openvue/inputtext`
- 位置：`@/components/ui/FpInput.vue`

## 用途

所有单行文本输入场景。带 `label`/`error` 的 FloatLabel 输入，错误自动关联 ARIA。

## Props

| 名 | 类型 | 默认 | 说明 |
|----|------|------|------|
| `label` | `string` | `''` | 浮动标签 |
| `error` | `string` | `''` | 错误文案（非空时 `aria-invalid` + `aria-describedby`） |
| `invalid` | `boolean` | `false` | 手动非法态 |
| `placeholder` | `string` | `''` | 占位文本 |
| `type` | `string` | `'text'` | 输入类型（`password` / `email` 等） |
| `toggleMask` | `boolean` | `false` | 密码可见性切换 |
| `disabled` | `boolean` | `false` | 禁用 |
| `readonly` | `boolean` | `false` | 只读 |

其余原生 input 属性（`maxlength`、`autocomplete`、`inputmode` 等）经 `$attrs` 透传。

## v-model

`v-model` 绑定 `string` 值。

## 示例

```vue
<FpInput v-model="form.username" :label="t('user.username')" :error="formErrors.username" />
```
