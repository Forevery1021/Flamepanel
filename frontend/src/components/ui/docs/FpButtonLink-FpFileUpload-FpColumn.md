# FpButtonLink / FpFileUpload / FpColumn

## FpButtonLink

文本链接按钮（原 `Button text`）。

- 底层：`openvue/button`

| 名 | 类型 | 默认 | 说明 |
|----|------|------|------|
| `label` | `string` | `''` | 文案 |
| `text` | `boolean` | `true` | 文本样式 |
| `size` | `'small' \| 'large'` | `'small'` | 尺寸 |

```vue
<FpButtonLink @click="edit(row)">{{ t('common.edit') }}</FpButtonLink>
```

## FpFileUpload

文件上传。

- 底层：`openvue/fileupload`；无额外 props（透传 `multiple`/`accept`/`auto`/`customUpload` 等）。

```vue
<FpFileUpload :custom-upload="true" @select="onSelect" />
```

## FpColumn

表格列定义，透传至 OpenVue `Column`。

- 底层：`openvue/column`；props 全部透传（`field`/`header`/`style`/`frozen`/`align-frozen`/`sortable` 等）。

```vue
<FpColumn field="name" :header="t('docker.name')" style="min-width: 140px">
  <template #body="{ data }">
    <span>{{ data.name }}</span>
  </template>
</FpColumn>
```
