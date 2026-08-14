# Fp* UI 组件库（components/ui）

> F2.2：业务 `views/` **禁止**直接 import OpenVue 底层组件，一律经 `@/components/ui` 的 Fp* 封装。除本页登记的例外（Terminal/xterm、ECharts、xterm.js）外，底层组件只允许出现在 `components/ui/` 内。

## 统一出口

```ts
import { FpButton, FpTable, FpModal, FpInput, useFpToast } from '@/components/ui'
```

`components/ui/index.ts` 导出全部 Fp* 组件与 hooks。

## 组件清单

| 组件 | 职责 / Props 要点 | 底层 |
|------|-------------------|------|
| `FpButton` | 语义变体 `primary/secondary/danger/success/warning/ghost/link`、8 态样式、loading | openvue/button |
| `FpButtonLink` | 文本链接按钮（原 `Button text`） | openvue/button |
| `FpCard` | 品牌标题栏 + 卡片容器 | openvue/card |
| `FpTable` | 统一空态/加载/分页/虚拟滚动/客户端排序；`rows/loading/virtual/sortable` | openvue/datatable |
| `FpColumn` | 表格列；`field/header/body slot` 透传 | openvue/column |
| `FpPagination` | 分页条；`total/page-size/first` + `@update:first`；total<=pageSize 自动隐藏 | openvue/paginator |
| `FpModal` | 重玻璃弹窗；`v-model` + `header/footer slot` | openvue/dialog |
| `FpDrawer` | 侧边抽屉；`v-model` + `position` | openvue/drawer |
| `FpInput` | FloatLabel 文本输入；`label/error/align` | openvue/inputtext |
| `FpSelect` | 下拉选择；`label/error/filter/multiple` | openvue/select |
| `FpNumber` | 数字输入；`label/error` | openvue/inputnumber |
| `FpTextarea` | 多行文本 | openvue/textarea |
| `FpSwitch` | 布尔开关；`v-model` | openvue/toggleswitch |
| `FpCheckbox` | 复选框；`v-model` + `change` | openvue/checkbox |
| `FpSlider` | 滑块；`v-model` | openvue/slider |
| `FpSelectButton` | 单选按钮组；`v-model` + `options` | openvue/selectbutton |
| `FpRadioGroup` / `FpRadioOption` | 单选组（支持 options 或自定义插槽） | openvue/radiobuttongroup |
| `FpTabs` | 声明式页签：`:items` + 命名 slot（`#tabValue`）；`v-model` | openvue/tabs |
| `FpDivider` | 分隔线 | openvue/divider |
| `FpTag` | 语义状态徽标 `success/warning/danger/info/neutral` + 运行点 | openvue/tag |
| `FpSkeleton` | 骨架屏 | openvue/skeleton |
| `FpProgress` | 进度条 | openvue/progressbar |
| `FpEmpty` | 空态（图标+标题+描述+操作） | — |
| `FpStatePanel` | loading/error(可重试)/empty 三态容器 | — |
| `FpBreadcrumb` | 面包屑 | openvue/breadcrumb |
| `FpFileUpload` | 文件上传 | openvue/fileupload |
| `FpChip` | 标签块 | openvue/chip |
| `FpInlineMessage` | 行内提示 | openvue/inlinemessage |
| `FpFormField` | 表单字段容器：label + slot + error/hint | — |
| `LayoutContent` | 页面布局（标题+工具栏+主体） | — |

## Hooks

- `useFpToast()` → `{ success, error, warning, info }`（错误优先展示后端 message）
- `useFpConfirm()` → `{ confirmAction(options) }`（危险操作红按钮 + i18n）

## API 文档

每个 Fp 组件均有一份 **props/events/slots 文档**（Modernization 补齐，对应 OpenVue 现代化 §22.1）：

- 索引：[`docs/README.md`](./docs/README.md)
- 覆盖：`FpButton` `FpTable` `FpModal` `FpDrawer` `FpPagination` `FpTabs` `FpStatePanel` `FpInput` `FpSelect` `FpTextarea` `FpNumber` `FpFormField` `FpSwitch` `FpCheckbox` `FpSlider` `FpSelectButton` `FpRadioGroup` `FpTag` `FpEmpty` `FpSkeleton` `FpProgress` `FpBreadcrumb` `FpChip` `FpInlineMessage` `FpDivider` `FpCard` `FpButtonLink` `FpFileUpload` `FpColumn` + hooks `useFpToast` / `useFpConfirm`

> 维护约定：新增/修改 Fp 组件 props/events/slots 时，同步更新对应 `docs/*.md`。

## 规范

1. **所有 Fp 组件只消费 CSS 变量/令牌**（`--fp-*`），禁止写死 `#rrggbb`。
2. **views 禁止直接 import openvue**；确有 Fp 未覆盖的底层组件时，先补齐 Fp 封装并在本 README 登记。
3. **FpTable 表格页范式**：`<LayoutContent><FpTable :rows :loading><FpColumn .../></FpTable><FpPagination/></LayoutContent>`。
4. **FpTabs 写法**：

```vue
<FpTabs v-model="tab" :items="tabItems">
  <template #containers>…</template>
  <template #images>…</template>
</FpTabs>
<script setup lang="ts">
const tabItems: FpTabItem[] = [
  { value: 'containers', label: t('docker.containers') },
  { value: 'images', label: t('docker.images') },
]
</script>
```

## 已登记例外（views 直接使用底层）

- `TerminalView`：`@xterm/xterm` + `@xterm/addon-fit`（xterm.js 非 OpenVue 组件）
- `DashboardView`：`echarts`（图表库，非 OpenVue）
