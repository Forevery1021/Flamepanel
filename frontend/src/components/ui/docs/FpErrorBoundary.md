# FpErrorBoundary

可复用的渲染错误边界（R-2 交付）。捕获子树渲染错误，替换为可恢复的错误面板。

- 位置：`@/components/ui/FpErrorBoundary.vue`
- 关联：`Doc/23` §2.5 R-2、§7.1 封装层设计约束

## 用途

**分层错误边界**：页面组件崩溃时不应牵连整个面板——用本组件包住 `router-view`，
崩溃只替换内容区，侧栏/顶栏/页签保留，用户可切菜单自救。

| 边界位置 | 兜住什么 | 崩溃后表现 |
|----------|----------|------------|
| `App.vue`（顶层） | Layout 自身、路由层致命错误 | 全屏错误面板（此时确无骨架可保留） |
| `Layout.vue` 内容区 | 单个页面组件崩溃 | 仅内容区替换，**导航骨架保留** |

## Props

| 名 | 类型 | 默认 | 说明 |
|----|------|------|------|
| `report` | `boolean` | `true` | 是否经 `utils/monitor` 全局上报 |
| `resetKey` | `string \| number` | — | 复位键：值变化即清除错误态（通常传 `route.fullPath`，切页不残留错误面板） |

## Events

无（错误经 `utils/monitor` 的 `reportError` 上报，不向父组件抛事件）。

## Slots

| 名 | 说明 |
|----|------|
| `default` | 被保护的子树；边界捕获到错误后由错误面板替换 |

## 设计约束

1. **零第三方依赖**：面板用原生 `<button>` + 设计令牌自绘，不经 OpenVue（`FpButton`）。
   兜底层必须比它保护的对象更稳——若崩溃源恰是 beta 依赖（Doc/23 P2-7），依赖它的
   兜底面板会同归于尽。
2. **文案自证**：面板文案走 `useSafeText`（`@/composables/useSafeText`），i18n 语言包
   未加载时回落 `@/utils/fallbackText` 的硬编码三语文案，绝不透出裸 key（`common.failed`）。
3. **不读 router**：复位靠 `resetKey` 外部驱动，组件本身不依赖路由，可在任意上下文复用。
4. **阻止冒泡**：`onErrorCaptured` 中 `return false`，同一次错误不会重复触发
   `app.config.errorHandler`，也不会再被上层边界二次接管。

## 示例

```vue
<!-- Layout.vue：页面级边界（保留导航骨架） -->
<main class="main">
  <FpErrorBoundary :reset-key="route.fullPath">
    <router-view v-slot="{ Component }">
      <component :is="Component" />
    </router-view>
  </FpErrorBoundary>
</main>
```

## 回归保护

`__tests__/FpErrorBoundary.spec.ts`（7 例）：正常子树 / setup 崩溃 / 渲染崩溃 /
文案可见性（非裸 key）/ 重试恢复 / resetKey 复位 / report 开关。
`components/__tests__/Layout.spec.ts`（3 例）：锁定「页面崩溃保留骨架」这一验收点。
