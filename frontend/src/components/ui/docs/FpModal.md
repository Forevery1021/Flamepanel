# FpModal

统一模态弹窗（重玻璃浮动层）。

- 底层：`openvue/dialog`
- 位置：`@/components/ui/FpModal.vue`

## 用途

创建/编辑/确认等对话框场景。

## Props

| 名 | 类型 | 默认 | 说明 |
|----|------|------|------|
| `header` | `string` | `''` | 标题 |
| `closable` | `boolean` | `true` | 是否显示关闭按钮 |
| `dismissableMask` | `boolean` | `true` | 点击遮罩关闭 |

其余 `openvue/dialog` 属性（`width`、`maximizable` 等）经 `$attrs` 透传。

## v-model

`v-model` 绑定 `boolean` 控制显隐（必填）。

## Slots

| 名 | 说明 |
|----|------|
| `default` | 弹窗主体 |
| `header` | 自定义标题 |
| `footer` | 底栏（通常放取消 + 主操作按钮） |

## 示例

```vue
<FpModal v-model="dialogVisible" :header="t('user.createUser')">
  <FpInput v-model="form.username" :label="t('user.username')" />
  <template #footer>
    <FpButton variant="secondary" @click="dialogVisible = false">{{ t('common.cancel') }}</FpButton>
    <FpButton variant="primary" :loading="saving" @click="save">{{ t('common.save') }}</FpButton>
  </template>
</FpModal>
```
