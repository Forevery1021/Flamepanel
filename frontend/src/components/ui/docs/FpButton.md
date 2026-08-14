# FpButton

统一按钮：语义化变体、8 态样式、loading、图标。

- 底层：`openvue/button`
- 位置：`@/components/ui/FpButton.vue`

## 用途

列表操作、表单提交、工具栏主操作等所有按钮场景的统一入口。
禁止在业务视图直接使用底层 OpenVue `Button`。

## Props

| 名 | 类型 | 默认 | 说明 |
|----|------|------|------|
| `variant` | `'primary' \| 'secondary' \| 'danger' \| 'success' \| 'warning' \| 'ghost' \| 'link'` | `'primary'` | 语义化变体，映射到 OpenVue severity（ghost→secondary+plain，link→text） |
| `label` | `string` | `''` | 按钮文案 |
| `icon` | `string` | `''` | openicons 图标类名（如 `oi oi-plus`） |
| `iconPos` | `'left' \| 'right'` | `'left'` | 图标位置 |
| `size` | `'small' \| 'large'` | `'small'` | 尺寸 |
| `loading` | `boolean` | `false` | 加载态（禁用 + spinner） |
| `disabled` | `boolean` | `false` | 禁用 |
| `type` | `ButtonHTMLAttributes['type']` | `'button'` | 原生 type |
| `plain` | `boolean` | `false` | 朴素样式 |
| `rounded` | `boolean` | `false` | 圆角按钮 |
| `outlined` | `boolean` | `false` | 描边样式 |
| `text` | `boolean` | `false` | 文本按钮 |
| `title` | `string` | `''` | 原生 title / tooltip |

## Events

透传 OpenVue `Button` 的 `click` 等原生事件（`@click` 直接在组件上监听）。

## Slots

| 名 | 说明 |
|----|------|
| `default` | 按钮内容（优先于 `label` 使用） |

## 示例

```vue
<FpButton variant="primary" icon="oi oi-plus" @click="openCreate">
  {{ t('common.create') }}
</FpButton>

<FpButton variant="danger" :loading="deleting" @click="confirmDelete(row)">
  {{ t('common.delete') }}
</FpButton>
```
