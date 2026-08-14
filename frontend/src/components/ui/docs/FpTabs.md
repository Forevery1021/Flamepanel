# FpTabs

统一声明式页签。

- 底层：`openvue/tabs`（TabList / Tab / TabPanels / TabPanel）
- 位置：`@/components/ui/FpTabs.vue`

## 用途

多页签视图（Docker / Settings / AppStore 等）。`:items` 声明式 + 命名 slot（`#tab{Value}`）。

## 类型

```ts
interface FpTabItem {
  value: string
  label: string
  icon?: string
}
```

## Props

| 名 | 类型 | 默认 | 说明 |
|----|------|------|------|
| `items` | `FpTabItem[]` | `[]` | 页签项 |

## v-model

`v-model` 绑定 `string` 当前激活的 `value`。

## Slots

| 名 | 说明 |
|----|------|
| `#tab{Value}` | 每个页签内容面板，按 `value` 命名（如 `#tabcontainers`） |

## 示例

```vue
<FpTabs v-model="tab" :items="tabItems">
  <template #tabcontainers>
    <!-- containers 面板 -->
  </template>
  <template #tabimages>
    <!-- images 面板 -->
  </template>
</FpTabs>
```
