# FlamePanel 前端重构设计文档（design.md）

> 依据：`flamepanel-design-system` 参考方案 + Impeccable 设计准则 + Taste Skill 反模板规则
> 技术栈决策：**UI 库替换为 OpenVue**（PrimeVue v4 社区延续版，MIT），替代参考方案中的 OpenTiny

---

## 1. 背景与目标

FlamePanel 是 Rust 后端 + Vue 3 前端的服务器运维面板（类似 1Panel），现有 23 个视图 + 6 步 Setup 向导、3 语言（zh/en/ja）。当前 UI 为 Element Plus 默认风格 + 少量手写令牌，视觉平庸、组件交互单一（无 loading/error/empty 全态覆盖）、无主题定制能力。

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
  - 品牌色三滑块（色相 0-360 / 彩度 / 亮度）→ JS 只发布 **OKLCH 种子** `--fp-brand-l/c/h`，色阶（`--fp-brand-50…900`）、`--fp-brand-strong/soft` 与自动对比前景 `--fp-brand-fg` 全部由 CSS `oklch(from …)` 相对色语法派生（不支持的浏览器回退到 JS 写死的三档）
  - 玻璃模糊度滑块（0-24px）、圆角大小（锐/中/圆三档）、密度模式（舒适/标准/紧凑）
  - **Export JSON** 导出定制配置，Import 可还原
- 切换机制：`[data-theme="flame|aurora|infinity"]` + `.dark` 类；OpenVue 预设 `darkModeSelector: '.dark'`

## 5. 材质与造型规则

- **三档材质**（第三轮起）：`solid`（纯色）/ `glass`（默认，轻玻璃）/ `acrylic`（重模糊 + 高饱和 + 内嵌噪点）。由 `stores/theme.ts` 写入 `<html data-material="…">`，持久化 `flamepanel.material`（自动迁移旧的 `flamepanel.glass` 布尔）。
- **组件只消费 5 个 `--fp-surface-*` 令牌**（`bg / bg-heavy / border / blur / highlight`），材质切换不改任何组件样式；`glass.css` 是这些令牌的唯一消费者。
- **玻璃/亚克力仅用于浮动层**：顶栏（轻玻璃）、Modal/Drawer（重玻璃）、Popover/Tooltip、⌘K 命令面板（重玻璃）
- 侧边栏实色 + 细边框（不透明，保证长列表滚动性能）
- `prefers-reduced-transparency` 媒体查询下强制 `solid`（优先级高于用户选择）
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
│  分组菜单     │  （Dashboard/…23 视图）       │
│  220px/64px  │                             │
├──────────────┴─────────────────────────────┤
│  AppFooter（版本号 + 服务器状态，可选）        │
└────────────────────────────────────────────┘
```

- **AppHeader**：折叠按钮、面包屑、WS 连接状态徽标、语言切换、主题切换（明/暗）、用户下拉（含「重新播放引导」）
- **AppSidebar**：品牌 Logo（火焰标记 + FlamePanel）、分组菜单（沿用现有 5 组：网站/应用/容器与存储/安全与运维/系统）、**底部固定「面板设置」一级入口**（`pinned: 'bottom'`，快捷键 `,`）、折叠态 64px、移动端转 Drawer
- **无多页签栏**（第三轮移除）：keep-alive 白名单直接来自路由 `meta.viewName`，路由即状态，减少一层"页面栈"心智
- **CommandPalette（⌘K）**：全局命令面板，支持菜单跳转 + 页面内操作（重启/部署等注册的快捷命令），重玻璃材质，`/` 或 `Ctrl+K` 唤起
- 移动端：`< 768px` 侧边栏收起为抽屉，表格横向滚动，卡片单列

## 10. 页面迁移清单（23 视图 + Setup）

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

---

## 16. UI 交互重构 · 第二轮（2026-09-16，feat/frontend-ui-refresh）

> 依据：全站 24 页真实截图诊断（`shots/before/`）。第一轮迁移完成了「换库」，本轮解决「像产品」。
> OpenVue 理念延续：**组件库负责行为与可达性，设计系统负责外观**——所有改动落在 `--fp-*` 令牌、Fp* 封装层与页面骨架，业务视图只换范式不换逻辑。

### 16.1 诊断结论（修什么）

| # | 现象 | 根因 | 处置 |
|---|---|---|---|
| D1 | 列表页「创建 xx」主按钮被压成竖排单字 | `FpButton` 无 `white-space:nowrap / flex-shrink:0`，toolbar 用 `space-between + wrap` | FpButton 基线修复（全站生效） |
| D2 | 进入 `/terminal` 后被登出 | `utils/ws.ts` 在 `onopen` **无条件** `getRefreshedToken()`，refresh 失败 → `clearAuth()` | WS 侧只在 access token 剩余 < 60s 时刷新；刷新失败不清凭证，交由 HTTP 401 链路裁决 |
| D3 | 网络 IO 图 X 轴标签重叠 | 默认 `axisLabel` 无 `hideOverlap` + 时间格式过长 | 图表 option 统一 `hideOverlap: true`、`HH:mm:ss` |
| D4 | 趋势面板 500px 高、下 2/3 空白 | 固定高度 | 图表容器改为 `aspect-ratio` + `min-height`，右栏两卡对齐 |
| D5 | 空态整屏无引导 | `FpStatePanel` 空态无 action 直通 | 空态支持 `empty-action` 插槽 + 页面传入「创建」主操作 |

### 16.2 设计语言微调（三拨盘不变，执行升级）

- **层级**：应用底 `--fp-bg-app` → 页面区块 `--fp-bg-elevated` → 卡内分区 `--fp-bg-subtle`（新增令牌）。取消"卡片套卡片"。
- **页头**：标题 20px/700 + 一行 `description`（新增 prop）+ 右侧主操作固定不换行；标题下方可选 **摘要条**（`#summary` 插槽：总数 / 运行中 / 异常 的 KPI pill）。
- **品牌橙**：暗色主按钮改用 `--fp-brand` 底 + `--fp-text-invert`，hover 提亮 6%、active 压暗 6% + `translateY(1px)`；大面积橙仅出现在主按钮与当前导航项。
- **导航**：默认展开当前路由所在分组；分组标题小型大写字母风格（11px/600/0.04em）；当前项左侧 2px 品牌色指示条替代整块填充。
- **顶栏**：保留 折叠 · 面包屑 · ⌘K · 状态徽标组 · 用户；语言/主题合并进用户菜单（减少 2 个图标）。多页签栏保留（可在外观设置关闭）。
- **数据卡片**：标签在上（12px muted）、数值 28px/700 等宽数字 `font-variant-numeric: tabular-nums`、右上角迷你趋势（sparkline，取 history 末 30 点）、底部 3 段色阶进度（<60 绿 / <85 黄 / ≥85 红）。
- **表格页范式**（`LayoutContent` + `PageToolbar` + `FpStatePanel` + `FpTable`）：
  - 工具栏：左搜索（带 ⌕ 图标、`/` 快捷聚焦）+ 筛选；右主操作 + 次操作 + 刷新
  - 行操作：≤2 个直接显示 link 按钮，≥3 个收进「⋯」菜单
  - 空态：图标 + 标题 + 一句说明 + 主操作按钮
  - 骨架：表格 loading 显示 5 行骨架而非 spinner
- **动效**：页面切换 160ms 淡入 + 6px 上移；卡片 hover 仅 `border-color` + `translateY(-1px)`；全部受 `prefers-reduced-motion` 约束。

### 16.3 实施顺序

P0 缺陷（D1–D5）→ 令牌/FpButton/LayoutContent/PageToolbar/FpStatePanel 基线 → AppSidebar/AppHeader → Dashboard → 列表页（按范式逐页）→ 复杂页 → 门禁 + 截图回归。

## 17. 第三轮 · 性能与开屏体验（2026-09-24，feat/ux-round3）

> 目标：开屏更快、更"像一个面板"；去掉多页签；材质与 OKLCH 语言落到令牌层；安装向导与新手引导补齐"第一次使用"的体验闭环；面板设置升为一级菜单。
> 后端同步重构指标采集/推送链路（记录见 `CHANGELOG.md`，链路说明见 `Doc/01-架构设计.md` §3.4）。

### 17.1 开屏（Dashboard）

| 手段 | 结果 |
|---|---|
| 三个图表面板改为 `defineAsyncComponent` + `ChartSkeleton` 占位，echarts 按需注册收敛到 `echartsSetup.ts`（既有 `manualChunks` 的 `echarts` chunk 因此脱离首屏关键路径） | 首屏不再等 176KB(gz) 的图表库；统计卡先出、图表后到 |
| WS `init` 帧回填历史后立刻渲染；未到数据前统计卡显示 **骨架**（`.stat-skeleton`）而非误导性的 `0.0%` | 无"闪 0" |
| `useMetricsStream`：history 用 `shallowRef` + 定长环形写入；sparkline path 记忆化（仅末 30 点变化时重算） | 每 tick 只有 4 张卡 + 3 张图的最小重绘 |
| 图表更新经 `useChartThrottle` 合帧（rAF） | 高频 tick 不抖 |
| 首屏 gzip 预算 200KB：**178.2KB**（`npm run bundle:check`） | 通过 |

### 17.2 多页签移除（UX-1）

- 删除 `AppTabs.vue` / `stores/tabs.ts` 与 `tabs.*`、`settingsAppearance.menuTabs*` i18n 键（三语）。
- `Layout.vue` 的 `<KeepAlive :include>` 改为从 `useRouter().getRoutes()` 读取 `meta.viewName`（不 import 路由单例，方便测试注入内存路由）。
- 后端 `open_menu_tabs` 设置项已随 Doc/30 S-04 删除（默认设置移除 + 幂等迁移清理旧库）。

### 17.3 材质与 OKLCH（见 §4.3 / §4.4 / §5）

- `data-material="solid|glass|acrylic"` + `--fp-surface-*` 令牌；主题页新增材质选择卡；导出/导入 JSON 含 `material`。
- 品牌色只发种子 `--fp-brand-l/c/h`；`--fp-brand-hover/active`（ΔL ±0.04/−0.06，OKLCH 下不偏色）、`--fp-brand-50…900` 色阶（色相锁定、彩度两端收敛）、`--fp-brand-fg`（`oklch(from …)` 按 L 自动选深/浅前景）、`--fp-focus-ring`、`--fp-brand-gradient` 全部由 CSS 派生；暗色种子 L 高一档、`strong` 变为提亮。

### 17.4 面板设置一级化（UX-3）

- 路由 `settings` 归 `main` 组、`pinned: 'bottom'`，侧栏底部常驻；`,` 快捷键直达。
- `SettingsView` 左侧竖向 `role=tablist` 六分区（general / security / backup / dns / theme / appearance），URL hash 深链（`/settings#security`），`router.replace` 同步、↑↓ Home End 键盘导航。
- **脏状态**：逐分区标记未保存圆点；保存只发送 **变更键**，走原子 `PATCH /settings/batch`；离开页面有未保存改动时确认；重置回服务端快照。
- 分区内提供「重新播放新手引导」。

### 17.5 新手引导（UX-2）

- `stores/onboarding.ts`：按用户名持久化 `{version, outcome}`；`ONBOARDING_VERSION` 升级后自动重新触发；`autoStart(username)` 在 `Layout` 挂载时调用。
- `OnboardingTour.vue`：`Teleport` 到 body、SVG evenodd 聚光灯遮罩、`data-tour="…"` 锚点（sidebar / header / stats / trend / command / settings / done）。**锚点缺失自动跳步**（页面精简也不会卡死），`stats` 步自动跳转到 `/dashboard`。
- 可跳过：Esc / 「跳过」；← → / Enter 翻页；`prefers-reduced-motion` 去动效；**非阻塞**（遮罩 `pointer-events:none`，只有卡片可交互）。

### 17.6 安装向导（UX-4）

- 顶部进度条 + 可点击回退到已完成步骤；Enter 前进；每步自动聚焦首个输入。
- 管理员密码 4 段强度计（`utils/password.ts` 启发式 0–4 分，含常见弱口令/键盘序列惩罚）。
- `sessionStorage` 草稿（**不含密码**）：刷新后恢复到管理员步骤并提示"从上次继续 / 重新开始"。
- 完成页摘要新增面板端口与访问地址。

### 17.7 门禁（本轮本地执行情况）

- 前端：`typecheck` ✓ · `lint` ✓ · `vitest`（136 文件 / 1390 用例）✓ · `locale:check` ✓ · `build` ✓ · `bundle:check` 178.2KB/200KB ✓
- 后端：`cargo fmt --check` ✓ · `cargo check`（lib 与 `--all-targets`）0 warning ✓ · `cargo test`/`clippy` 视 CI（本地 2GB 沙箱无法完成全量编译，见 CHANGELOG）
- E2E（Playwright）：沙箱无浏览器，未运行；已按 `e2e/helpers/flow.ts` 约定**不新增 `data-testid`**，选择器全部走 role / id / 语义 class。
