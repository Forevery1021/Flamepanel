# FpSwitch / FpCheckbox / FpSlider

统一布尔开关、复选、滑块。

## FpSwitch

- 底层：`openvue/toggleswitch`

### Props

| 名 | 类型 | 默认 | 说明 |
|----|------|------|------|
| `disabled` | `boolean` | `false` | 禁用 |

### v-model

`v-model` 绑定 `boolean`。

```vue
<FpSwitch :model-value="data.enabled" @update:model-value="(v) => onToggle(data, v)" />
```

## FpCheckbox

- 底层：`openvue/checkbox`

### Props

| 名 | 类型 | 默认 | 说明 |
|----|------|------|------|
| `binary` | `boolean` | `true` | 布尔模式（单值） |

### v-model / Events

`v-model` 绑定 `boolean`；`@change(value: boolean)`——**载荷是本次交互后的新值**：勾选为 `true`，
取消勾选为 `false`（C-3 修复；历史实现误把底层 DOM Event 布尔化，导致取消勾选恒传 `true`）。

> 需要「勾选即触发」的双向操作（如待办的标记完成 / 取消完成）用 `@change`，
> 载荷直接使用即可，无需再 `Boolean(v)` 转换。

```vue
<FpCheckbox v-model="remember" binary />
<FpCheckbox :model-value="item.done" @change="(v) => toggle(item, v)" />
```

## FpSlider

- 底层：`openvue/slider`

### v-model

`v-model` 绑定 `number`。

```vue
<FpSlider v-model="radius" :min="0" :max="24" />
```
