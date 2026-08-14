# FpSkeleton / FpProgress / FpBreadcrumb / FpChip / FpInlineMessage / FpDivider / FpCard

统一轻量展示组件。

## FpSkeleton

- 底层：`openvue/skeleton`；无 props（全部透传 `width`/`height`/`shape` 等）。

```vue
<FpSkeleton width="100%" height="3rem" />
```

## FpProgress

- 底层：`openvue/progressbar`

| 名 | 类型 | 默认 | 说明 |
|----|------|------|------|
| `value` | `number` | `0` | 进度 0–100 |

```vue
<FpProgress :value="usage" />
```

## FpBreadcrumb

- 底层：`openvue/breadcrumb`

| 名 | 类型 | 默认 | 说明 |
|----|------|------|------|
| `items` | `MenuItem[]` | `[]` | 面包屑项 |

```vue
<FpBreadcrumb :items="[{ label: 'Home', to: '/' }, { label: 'Files' }]" />
```

## FpChip

- 底层：`openvue/chip`

| 名 | 类型 | 默认 | 说明 |
|----|------|------|------|
| `label` | `string` | `''` | 标签文案 |
| `removable` | `boolean` | `false` | 可移除 |

## FpInlineMessage

- 底层：`openvue/inlinemessage`

| 名 | 类型 | 默认 | 说明 |
|----|------|------|------|
| `severity` | `'info' \| 'success' \| 'warn' \| 'error'` | `'info'` | 语义色 |

```vue
<FpInlineMessage severity="error">{{ t('log.connectionFailed') }}</FpInlineMessage>
```

## FpDivider

- 底层：`openvue/divider`；无 props（透传 `layout` 等）。

```vue
<FpDivider />
```

## FpCard

- 底层：`openvue/card`

| 名 | 类型 | 默认 | 说明 |
|----|------|------|------|
| `title` | `string` | `''` | 卡片标题 |

```vue
<FpCard :title="t('settings.theme')">
  <slot />
</FpCard>
```
