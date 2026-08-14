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

`v-model` 绑定 `boolean`；`@change(value: boolean)`。

```vue
<FpCheckbox v-model="remember" binary />
```

## FpSlider

- 底层：`openvue/slider`

### v-model

`v-model` 绑定 `number`。

```vue
<FpSlider v-model="radius" :min="0" :max="24" />
```
