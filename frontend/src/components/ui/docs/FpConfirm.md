# useFpConfirm

统一确认弹窗（危险操作红按钮）。

- 底层：`openvue/useconfirm`
- 位置：`@/components/ui/FpConfirm.ts`

## 用途

删除、清空等危险/不可逆操作的二次确认。

## FpConfirmOptions

| 字段 | 类型 | 默认 | 说明 |
|------|------|------|------|
| `message` | `string` | —（必填） | 确认文案 |
| `header` | `string` | `'确认操作'` | 标题 |
| `icon` | `string` | `'oi oi-exclamation-triangle'` | 图标 |
| `acceptLabel` | `string` | `'确定'` | 确认按钮文案 |
| `rejectLabel` | `string` | `'取消'` | 取消按钮文案 |
| `accept` | `() => void` | — | 确认回调 |
| `reject` | `() => void` | — | 取消回调 |
| `danger` | `boolean` | `true` | 危险操作（红按钮）；`false` 为主色按钮 |

## 示例

```ts
const { confirmAction } = useFpConfirm()
confirmAction({
  message: t('user.deleteConfirm', { name: row.username }),
  accept: () => deleteUser(row.id),
})
```
