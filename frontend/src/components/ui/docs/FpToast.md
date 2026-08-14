# useFpToast

统一操作反馈（Toast 消息单例）。

- 底层：`openvue/usetoast`
- 位置：`@/components/ui/FpToast.ts`

## 用途

所有异步操作的成败反馈。错误优先展示后端/API 错误消息（复用 `getErrorMessage`），新消息顶掉旧消息防堆积。

## 返回值

| 方法 | 签名 | 说明 |
|------|------|------|
| `success` | `(message: string, title = '')` | 成功提示（3s） |
| `error` | `(err: unknown, fallback = '操作失败', title = '')` | 错误提示（4s，自动取后端 message） |
| `warning` | `(message: string, title = '')` | 警告（3.5s） |
| `info` | `(message: string, title = '')` | 信息（3s） |

## 示例

```ts
const toast = useFpToast()
try {
  await api.startContainer(id)
  toast.success(t('docker.startSuccess'))
} catch (e) {
  toast.error(e, t('docker.startFailed'))
}
```
