# Fp* 组件 API 文档索引

> 每组件一份 props/events/slots 文档（Modernization 队列补齐，对应 OpenVue 现代化 §22.1「每个 Fp 一份」）。

## 核心交互

| 组件 | 文档 | 用途 |
|------|------|------|
| `FpButton` | [FpButton.md](./FpButton.md) | 语义按钮、loading、图标 |
| `FpTable` | [FpTable.md](./FpTable.md) | 统一表格：分页/虚拟/排序 |
| `FpModal` | [FpModal.md](./FpModal.md) | 弹窗（重玻璃） |
| `FpDrawer` | [FpDrawer.md](./FpDrawer.md) | 侧边抽屉 |
| `FpPagination` | [FpPagination.md](./FpPagination.md) | 分页条 |
| `FpTabs` | [FpTabs.md](./FpTabs.md) | 声明式页签 |
| `FpStatePanel` | [FpStatePanel.md](./FpStatePanel.md) | 三态容器 |

## 表单

| 组件 | 文档 | 用途 |
|------|------|------|
| `FpInput` | [FpInput.md](./FpInput.md) | 文本输入 |
| `FpSelect` | [FpSelect.md](./FpSelect.md) | 下拉选择 |
| `FpTextarea` | [FpTextarea.md](./FpTextarea.md) | 多行文本 |
| `FpNumber` | [FpNumber.md](./FpNumber.md) | 数字输入 |
| `FpFormField` | [FpFormField.md](./FpFormField.md) | 表单字段容器 |
| `FpSwitch/FpCheckbox/FpSlider` | [FpSwitch-FpCheckbox-FpSlider.md](./FpSwitch-FpCheckbox-FpSlider.md) | 开关/复选/滑块 |
| `FpSelectButton/FpRadioGroup` | [FpSelectButton-FpRadioGroup.md](./FpSelectButton-FpRadioGroup.md) | 单选组 |

## 展示

| 组件 | 文档 | 用途 |
|------|------|------|
| `FpTag` | [FpTag.md](./FpTag.md) | 状态徽标 |
| `FpEmpty` | [FpEmpty.md](./FpEmpty.md) | 空态 |
| `FpSkeleton/FpProgress/FpBreadcrumb/FpChip/FpInlineMessage/FpDivider/FpCard` | [FpSkeleton-FpProgress-FpBreadcrumb-FpChip-FpInlineMessage-FpDivider-FpCard.md](./FpSkeleton-FpProgress-FpBreadcrumb-FpChip-FpInlineMessage-FpDivider-FpCard.md) | 轻量展示 |

## 其他

| 组件 | 文档 | 用途 |
|------|------|------|
| `FpButtonLink/FpFileUpload/FpColumn` | [FpButtonLink-FpFileUpload-FpColumn.md](./FpButtonLink-FpFileUpload-FpColumn.md) | 链接/上传/列 |

## Hooks

| Hook | 文档 | 用途 |
|------|------|------|
| `useFpToast()` | [FpToast.md](./FpToast.md) | 统一操作反馈 |
| `useFpConfirm()` | [FpConfirm.md](./FpConfirm.md) | 统一确认弹窗 |

## 维护约定

- 新增/修改 Fp 组件 props/events/slots 时，同步更新对应 `docs/*.md`。
- 新增 Fp 组件时，在本索引 + `components/ui/README.md` 登记，并新建一份文档。
