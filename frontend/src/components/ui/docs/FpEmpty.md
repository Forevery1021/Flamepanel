# FpEmpty

统一空态。

- 底层：自绘（图标 + 标题 + 描述 + 操作）
- 位置：`@/components/ui/FpEmpty.vue`

## 用途

无数据时的友好空态占位，可带操作按钮。

## Props

| 名 | 类型 | 默认 | 说明 |
|----|------|------|------|
| `icon` | `string` | `'oi oi-inbox'` | openicons 图标类名 |
| `title` | `string` | `''` | 空态标题 |
| `description` | `string` | `''` | 空态描述 |

## Slots

| 名 | 说明 |
|----|------|
| `default` | 空态主体内容（可含操作按钮） |
| `action` | 空态操作区（按钮等） |

## 示例

```vue
<FpEmpty :title="t('file.emptyDir')" icon="oi oi-folder-open">
  <template #action>
    <FpButton variant="primary" @click="upload">上传</FpButton>
  </template>
</FpEmpty>
```
