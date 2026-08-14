# FpNumber

统一数字输入。

- 底层：`openvue/inputnumber` + `openvue/floatlabel`
- 位置：`@/components/ui/FpNumber.vue`

## Props

| 名 | 类型 | 默认 | 说明 |
|----|------|------|------|
| `label` | `string` | `''` | 浮动标签 |
| `error` | `string` | `''` | 错误文案 |
| `invalid` | `boolean` | `false` | 手动非法态 |

其余属性（`min`/`max`/`step` 等）经 `$attrs` 透传。

## v-model

`v-model` 绑定 `number | null`。

## 示例

```vue
<FpNumber v-model="form.port" :label="t('firewall.port')" :min="1" :max="65535" />
```
