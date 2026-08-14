# FpTag

统一语义状态徽标。

- 底层：`openvue/tag`
- 位置：`@/components/ui/FpTag.vue`

## 用途

状态、类型、运行点等徽标。

## Props

| 名 | 类型 | 默认 | 说明 |
|----|------|------|------|
| `value` | `string` | `''` | 徽标文案 |
| `severity` | `'success' \| 'warning' \| 'danger' \| 'info' \| 'neutral'` | `'neutral'` | 语义色（warning→warn、neutral→secondary） |
| `dot` | `boolean` | `false` | 显示运行点 |
| `rounded` | `boolean` | `true` | 圆角 |

## 示例

```vue
<FpTag
  :severity="data.status === 'running' ? 'success' : 'danger'"
  :value="data.status === 'running' ? t('database.running') : t('database.stopped')"
  dot
/>
```
