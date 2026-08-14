# FpDrawer

统一侧边抽屉。

- 底层：`openvue/drawer`
- 位置：`@/components/ui/FpDrawer.vue`

## 用途

详情 / 编辑等右侧（或指定方向）抽屉。

## Props

| 名 | 类型 | 默认 | 说明 |
|----|------|------|------|
| `header` | `string` | `''` | 标题 |
| `position` | `'left' \| 'right' \| 'top' \| 'bottom'` | `'right'` | 抽屉方向 |

其余 `openvue/drawer` 属性经 `$attrs` 透传。

## v-model

`v-model` 绑定 `boolean` 控制显隐（必填）。

## Slots

| 名 | 说明 |
|----|------|
| `default` | 抽屉内容 |
| `header` | 自定义标题 |
| `footer` | 底栏 |

## 示例

```vue
<FpDrawer v-model="detailVisible" :header="t('docker.detail')" position="right">
  <pre>{{ detail }}</pre>
  <template #footer>
    <FpButton variant="secondary" @click="detailVisible = false">关闭</FpButton>
  </template>
</FpDrawer>
```
