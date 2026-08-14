# FpTextarea

统一多行文本输入。

- 底层：`openvue/textarea`
- 位置：`@/components/ui/FpTextarea.vue`

## Props

| 名 | 类型 | 默认 | 说明 |
|----|------|------|------|
| `error` | `string` | `''` | 错误文案（`aria-invalid` + `aria-describedby`） |
| `invalid` | `boolean` | `false` | 手动非法态 |
| `placeholder` | `string` | `''` | 占位文本 |

其余原生 textarea 属性经 `$attrs` 透传。

## v-model

`v-model` 绑定 `string`。

## 示例

```vue
<FpTextarea v-model="form.command" :label="t('scheduledTask.command')" rows="4" />
```
