# FlamePanel 前端重构设计文档（design.md）

> 依据：`flamepanel-design-system` 参考方案 + Impeccable 设计准则 + Taste Skill 反模板规则
> 技术栈决策：**UI 库替换为 OpenVue**（PrimeVue v4 社区延续版，MIT），替代参考方案中的 OpenTiny

---

## 1. 背景与目标

FlamePanel 是 Rust 后端 + Vue 3 前端的服务器运维面板（类似 1Panel），现有 20 个视图、3 语言（zh/en/ja）、约 8000 行前端代码。当前 UI 为 Element Plus 默认风格 + 少量手写令牌，视觉平庸、组件交互单一（无 loading/error/empty 全态覆盖）、无主题定制能力。

**重构目标**：
- 全面视觉重塑，形成有辨识度的「火焰运维面板」设计语言
- 组件库从 Element Plus 迁移到 OpenVue（PrimeVue v4 API，80+ 组件，MIT）
- 建立设计令牌系统（OKLCH 色彩空间）+ 运行时主题定制（品牌色/玻璃/密度可调 + JSON 导出）
- 交互全状态覆盖（loading/empty/error/8 态按钮），数据密集场景虚拟滚动
- 性能预算达标（首屏 JS < 200KB gzip、主题切换 < 16ms）

## 2. 设计 Read（Impeccable）

> **Reading this as**: 面向系统管理员的服务器运维面板（Operate 模式），高密度数据界面，暗色优先、终端感强的技术语言，倾向于自定义 OpenVue Aura 预设 + 克制动效。

**三拨盘**：
- `DESIGN_VARIANCE: 6` — 中高变化：非对称留白、数据卡片错落，但不极端
- `MOTION_INTENSITY: 4` — 服务反馈不表演：hover/active/页面过渡/数字滚动，无花哨动画
- `VISUAL_DENSITY: 7` — 高密度：运维面板刚需，数据优先、留白克制

## 3. 技术选型决策

| 维度 | 选择 | 理由 |
|------|------|------|
| 组件库 | **OpenVue** (`openvue@beta`) | PrimeVue v4 延续版，80+ 组件，Aura/Lara 预设，设计令牌体系完整 |
| 主题包 | `@openvue/themes@beta` | 官方主题预设 + `darkModeSelector`，与现有 `.dark` 类切换机制兼容 |
| 图标 | `@openvue/openicons` | 官方图标库，单一图标族（禁用混用） |
| 原子化 CSS | **UnoCSS** + `presetWind4` | Tailwind v4 兼容语法，按需生成，与设计令牌 CSS 变量联动 |
| 构建 | Vite 8（现有） | 已就位，不改动 |
| 色彩空间 | OKLCH | 感知均匀，明暗主题切换无跳变 |
| 字体 | Geist（可变）+ JetBrains Mono + Noto Sans SC 回退 | 现代无衬线 + 等宽技术字体 + 中文兜底 |
| 包管理 | npm（现有） | 保持现状，不引入 pnpm |
| 后端代理 | `localhost:8080`（现有） | 修正参考方案中的 3000 端口 |

**与参考方案的差异**（`flamepanel-design-system/README.md`）：
- 组件库：OpenTiny → **OpenVue**（组件映射见 §7）
- 视图组织：参考方案按模块建子目录；**本方案保持现有扁平 `src/views/` 结构**，减少迁移 churn
- 新增入口：主题定制并入现有 `/settings` 视图（新增 Theme 页签），不新建路由
- 参考方案中的 audit 视图替换为实际存在的 OperationLogs/SystemLogs

## 4. 设计令牌系统

### 4.1 目录结构（新增）

```
src/
├── theme/
│   ├── tokens.css              # 语义设计令牌（--fp-* 前缀，OKLCH）
│   ├── flame-preset.ts         # OpenVue 自定义主题预设（基于 Aura）
│   ├── themes.ts               # 主题切换/定制 store 逻辑
│   └── glass.css               # 玻璃/亚克力材质工具类
├── components/
│   ├── ui/                     # Fp* OpenVue 封装层
│   ├── layout/                 # AppHeader / AppSidebar / AppFooter / CommandPalette
│   └── terminal/XTerm.vue      # 从 TerminalView 抽离的 xterm 封装
└── composables/
    ├── useTheme.ts             # 主题状态 → DOM/令牌
    └── useECharts.ts           # ECharts 明暗主题自适应
```

### 4.2 令牌分层

1. **原始令牌**（`tokens.css` 根变量）：`--fp-brand`、`--fp-accent`、`--fp-surface-*`、`--fp-text-*`、`--fp-radius-*`、`--fp-space-*`、`--fp-glass-blur` —— 全部 OKLCH，禁止硬编码 hex
2. **语义令牌**：`--fp-bg-app`、`--fp-bg-elevated`、`--fp-border`、`--fp-text-primary/secondary/muted`、状态色 `--fp-success/warning/danger/info`（暗色主题在 `.dark` 下覆写）
3. **OpenVue 预设桥接**：`flame-preset.ts` 把 `--fp-*` 注入 Aura 预设的设计令牌覆盖（`--p-*` 变量），实现组件库与站点语言同源

### 4.3 色彩

- 基础中性色：冷灰（slate 家族），暗色 `#0f1115` 底 + `#16181d` 卡片（沿用现有，微调至 OKLCH）
- 品牌色：**火焰橙**（`oklch(0.65 0.19 35)` ≈ #ea580c），象征 FlamePanel 命名，替代现有靛蓝
- 语义色：success `oklch(0.62 0.16 155)`、warning `oklch(0.75 0.15 75)`、danger `oklch(0.6 0.21 25)`、info 沿用中性蓝
- 单强调色原则：整页仅品牌橙 + 语义状态色，不混入第二强调色

### 4.4 主题体系（运行时定制）

- 内置 4 主题：**Flame**（默认，暗色优先）、**Aurora**（浅色）、**Infinity**（高对比暗色）、**Brand Custom**（用户定制）
- 主题状态：Pinia store（`stores/theme.ts`）持久化到 `localStorage`
- 定制能力（设置页 Theme 页签）：
  - 品牌色 HSL 三滑块（色相 0-360 / 饱和度 / 亮度）→ 实时生成 OKLCH 覆写
  - 玻璃模糊度滑块（0-24px）、圆角大小（锐/中/圆三档）、密度模式（舒适/标准/紧凑）
  - **Export JSON** 导出定制配置，Import 可还原
- 切换机制：`[data-theme="flame|aurora|infinity"]` + `.dark` 类；OpenVue 预设 `darkModeSelector: '.dark'`

## 5. 材质与造型规则

- **玻璃/亚克力仅用于浮动层**：顶栏（轻玻璃）、Modal/Drawer（重玻璃）、Popover/Tooltip、⌘K 命令面板（重玻璃）
- 侧边栏实色 + 细边框（不透明，保证长列表滚动性能）
- `prefers-reduced-transparency` 媒体查询下玻璃回退为纯色
- 圆角体系：控件 8px、卡片 12px、浮动层 16px，全站统一
- 阴影：品牌色相微染（`oklch` 带 alpha），禁止纯黑投影

## 6. 交互规范

- **8 态全覆盖**：default / hover / focus-visible / active / disabled / loading / error / success（按钮至少实现前 6 态）
- 触感反馈：`:active` 时 `translateY(1px)` 或 `scale(0.98)`
- 动效克制（MOTION 4）：页面切换淡入 + 6px 上移、数字增长滚动、图表入场；全站 `prefers-reduced-motion: reduce` 降级
- loading：骨架屏优先（表格/卡片用 Skeleton 占位），仅全局等待用 spinner
- 反馈：操作成功用 Toast（右下，3s 自动消失）；危险操作用 ConfirmDialog（`useConfirm`）；表单错误行内展示

## 7. 组件映射表（Element Plus → OpenVue）

| Element Plus | OpenVue | 备注 |
|---|---|---|
| `el-button` | `Button` | 经 FpButton 封装（8 态） |
| `el-table` / `el-table-column` | `DataTable` / `Column` | 经 FpTable 封装；`virtualScroller` 支持大列表 |
| `el-form` / `el-form-item` | `Fieldset` + `FloatLabel` + `Message` | 校验保留现有 `FormRules` 逻辑，迁移为本地校验函数 |
| `el-input` | `InputText` + `FloatLabel` | FpInput 封装 |
| `el-select` / `el-option` | `Select` / `Option` | FpSelect 封装 |
| `el-dialog` | `Dialog` | FpModal 封装（重玻璃） |
| `el-drawer` | `Drawer` | 直接用 openvue Drawer（中玻璃） |
| `ElMessage` | `useToast()` | FpToast 统一封装（195 处调用点） |
| `ElMessageBox.confirm` | `useConfirm()` | FpConfirm 统一封装（14 处调用点） |
| `el-card` | `Card` / `Panel` | Dashboard 卡片自定义布局 |
| `el-tag` | `Tag` | FpTag 语义色封装 |
| `el-tooltip` | `v-tooltip` 指令 | 全局注册，统一 showDelay=300 |
| `el-popconfirm` | `ConfirmPopup` | 行内危险操作 |
| `el-popover` | `Popover` | 保留原生使用 |
| `el-tabs` / `el-tab-pane` | `Tabs` / `TabPanel` | — |
| `el-row` / `el-col` | `Grid` / `Col` | v4 新版 Grid 语法 |
| `el-input-number` | `InputNumber` | — |
| `el-descriptions` | `Descriptions` | 详情页/信息展示 |
| `el-switch` | `ToggleSwitch` | — |
| `el-pagination` | `Paginator` | FpTable 内置 |
| `el-divider` | `Divider` | — |
| `el-checkbox` | `Checkbox` | — |
| `el-radio-group` | `SelectButton` / `RadioGroup` | 按场景选择 |
| `el-progress` | `ProgressBar` | Dashboard 环形用 `ProgressSpinner` |
| `el-alert` | `Message`（inline） | — |
| `el-upload` | `FileUpload` | — |
| `el-menu` / `el-menu-item` | 自研侧边栏 + `Menu` | AppSidebar 自绘，菜单弹层用 Menu |
| `el-breadcrumb` | `Breadcrumb` | — |
| `el-empty` | 自绘空态（FpEmpty） | 图标 + 引导文案 |
| `el-link` | `Button link` 变体 | — |
| `@element-plus/icons-vue` | `@openvue/openicons` | 全部替换 |

## 8. Fp* UI 封装层（components/ui/）

每个封装只做「设计系统约束 + 常用默认值」，透传其余 props：

| 组件 | 职责 |
|---|---|
| `FpButton.vue` | 8 态样式、尺寸规范、危险/主要/次要/幽灵变体 |
| `FpTable.vue` | DataTable 包装：统一空态、loading 骨架、分页、状态 Tag 列约定、虚拟滚动开关 |
| `FpModal.vue` | 重玻璃材质、统一动画与遮罩（Drawer 直接用 openvue 组件） |
| `FpInput.vue` / `FpSelect.vue` | FloatLabel 布局、错误态样式、disabled/loading |
| `FpTag.vue` | 语义状态色（success/warning/danger/info）+ 运行状态徽标 |
| `FpToast.ts` | `useToast` 封装：错误优先展示后端 message（复用 `getErrorMessage`） |
| `FpConfirm.ts` | `useConfirm` 封装：危险操作红按钮、i18n 默认文案 |
| `FpEmpty.vue` | 空态：图标 + 标题 + 引导操作 |

## 9. 布局架构

```
┌──────────────┬─────────────────────────────┐
│  AppSidebar  │  AppHeader（轻玻璃 56px）     │
│  实色+细边框   ├─────────────────────────────┤
│  FlamePanel  │  router-view（页面切换过渡）   │
│  分组菜单     │  （Dashboard/…20 视图）       │
│  220px/64px  │                             │
├──────────────┴─────────────────────────────┤
│  AppFooter（版本号 + 服务器状态，可选）        │
└────────────────────────────────────────────┘
```

- **AppHeader**：折叠按钮、面包屑、WS 连接状态徽标、语言切换、主题切换（明/暗）、用户下拉
- **AppSidebar**：品牌 Logo（火焰标记 + FlamePanel）、分组菜单（沿用现有 5 组：网站/应用/容器与存储/安全与运维/系统）、折叠态 64px、移动端转 Drawer
- **CommandPalette（⌘K）**：全局命令面板，支持菜单跳转 + 页面内操作（重启/部署等注册的快捷命令），重玻璃材质，`/` 或 `Ctrl+K` 唤起
- 移动端：`< 768px` 侧边栏收起为抽屉，表格横向滚动，卡片单列

## 10. 页面迁移清单（20 视图）

| 页面 | 迁移重点 |
|---|---|
| Dashboard | 数据卡片重设计（大数字 + 迷你进度 + 状态点）、ECharts 趋势/网络图适配、WS 状态徽标 |
| Login | 居中卡片 + 品牌火焰标记 + 背景微光，表单全态 |
| Users / Firewall / Nodes / ScheduledTasks / Backups / Databases | 标准 FpTable 页：搜索栏 + 工具栏 + 表格 + 分页 + 行内操作 |
| Docker / WebServers / Websites / AppStore / Plugins | 复杂表格 + Tabs + 状态 Tag 体系（运行中/已停止/异常） |
| Files | 面包屑 + 表格 + 上传/新建对话框 + 右键菜单（Menu） |
| Terminal | 抽离 XTerm.vue（xterm.js 现有集成不动），全屏模式 |
| Memos | 卡片流布局（替换表格） |
| Health | 检查项列表 + 状态徽标 + 描述列表 |
| OperationLogs / SystemLogs | 日志表格 + 筛选 + 行详情 Dialog |
| Settings | 保留现有设置项 + **新增 Theme 页签**（主题定制面板） |
| AppStore 安装向导 | 多步 Dialog（Step 指示器 + 表单） |

## 11. 性能预算与优化

| 指标 | 预算 | 手段 |
|---|---|---|
| 首屏 JS (gzip) | < 200KB | OpenVue 按需引入、路由懒加载（现有）、UnoCSS 按需生成 |
| 主题切换延迟 | < 16ms | 令牌只作用于 CSS 变量，不触发重渲染 |
| 万级表格 FPS | > 55 | DataTable `virtualScroller` + `lazy` 模式 |
| 卡片悬浮 | 无重排 | 动效仅 `transform`/`opacity` |
| Lighthouse | > 90 | 迁移完成后跑分验证 |

## 12. 无障碍

- 全站对比度 WCAG AA（正文 4.5:1，大字 3:1），暗/浅双主题分别验证
- `:focus-visible` 环（现有规则保留），键盘可完整操作表格/对话框
- 玻璃材质提供 `prefers-reduced-transparency` 纯色回退
- 动效遵守 `prefers-reduced-motion`

## 13. 迁移路线图（适配本仓库节奏）

| 阶段 | 内容 | 验证 |
|---|---|---|
| **P0 基座** | 依赖安装（openvue/themes/openicons/unocss/fontsource）、vite/main.ts 改造、tokens.css、flame-preset、主题切换 | `npm run build` + 页面骨架可渲染 |
| **P1 封装层** | Fp* 组件全套 + FpToast/FpConfirm | build + 类型检查 |
| **P2 布局** | AppHeader/AppSidebar/AppFooter/CommandPalette + Layout.vue 重写 | 明暗主题手工冒烟 |
| **P3 模式确立** | Login + Dashboard + Settings(基础) 三个模板页面迁移 | 确立表格/表单/卡片三套范式 |
| **P4 表格页** | Users/Firewall/Nodes/ScheduledTasks/Backups/Databases/OperationLogs/SystemLogs/Memos/Health（10 页） | 逐页 build + 冒烟 |
| **P5 复杂页** | Docker/WebServers/Websites/AppStore/Plugins/Files/Terminal（7 页） | 逐页功能回归 |
| **P6 增强** | 主题定制面板（Theme 页签）、ECharts 主题适配、动效系统、⌘K 命令 | 定制 → 导出 → 导入闭环 |
| **P7 收尾** | 响应式 + 双主题验收 + a11y 审计 + Lighthouse + 性能预算验证 | 全量回归 + 报告 |

## 14. YAGNI 清单（明确不做）

- 不引入 `@openvue/forms`（表单校验用现有手写规则迁移）
- 不引入 `@openvue/nuxt-module`（无 Nuxt）
- 不重写后端 API、不换 ECharts/xterm/axios
- 不做页面级虚拟滚动之外的复杂可视化
- 不新增权限/路由体系（沿用现有）

---

## 15. 验收记录（2026-08-04）

- [x] 19/19 页面无渲染错误（Playwright 冒烟，API 桩模拟）
- [x] 主题切换闭环：跟随系统 → 手动切换 → localStorage 持久化 → 刷新保持
- [x] 主题定制：4 预设 + HSL/玻璃/圆角/密度滑块 + JSON 导出/导入
- [x] ⌘K 命令面板：键盘导航 + 搜索跳转
- [x] 移动端：<768px 侧栏转抽屉，卡片单列
- [x] `npx vue-tsc --noEmit` 0 错误；`npm run lint` 0 警告；`npm run build` 成功
- [x] element-plus / @element-plus/icons-vue 依赖已移除
- [x] 首屏 JS gzip：151 KB（预算 < 200 KB）
