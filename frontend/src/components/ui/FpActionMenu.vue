<template>
  <FpButton
    ref="btnRef"
    variant="ghost"
    icon="oi oi-ellipsis-h"
    :aria-label="ariaLabel || t('common.more')"
    :title="ariaLabel || t('common.more')"
    aria-haspopup="menu"
    :aria-expanded="open || undefined"
    :disabled="!visibleItems.length"
    :loading="anyLoading"
    class="fp-action-menu__trigger"
    @click="toggle"
  />
  <TieredMenu
    ref="menuRef"
    :model="menuModel"
    popup
    append-to="body"
    class="fp-action-menu"
    @show="open = true"
    @hide="open = false"
  >
    <template #item="{ item, props: ip }">
      <a
        v-bind="ip.action"
        class="fp-action-menu__item"
        :class="{ 'fp-action-menu__item--danger': item.danger, 'p-disabled': item.disabled }"
      >
        <i v-if="item.icon" :class="[item.icon, 'fp-action-menu__icon']" />
        <span>{{ item.label }}</span>
      </a>
    </template>
  </TieredMenu>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import TieredMenu from 'openvue/tieredmenu'
import type { MenuItem } from 'openvue/menuitem'
import FpButton from './FpButton.vue'
import { getActivePinia } from 'pinia'
import { useAuthStore } from '@/stores/auth'
import { hasPermission, resolveRole } from '@/directives/permission'

/**
 * 行操作「更多」菜单（Doc/04 §表格）：
 * 表格每行保留 1–2 个高频主操作为按钮，其余收进本菜单，避免 8–11 个按钮挤成一排。
 *
 * - `perm`：与 v-permission 同一判定源（useAuthStore().role + hasPermission），无权限项**不渲染**（view 模式语义）
 * - `disabled`：保留在菜单里但置灰（业务态不可用，如「停止」对已停止容器）
 * - `danger`：删除类操作红字，并自动与上方用分隔线隔开
 * - 无可见项时触发按钮禁用
 */
export interface FpActionItem {
  key: string
  label: string
  icon?: string
  perm?: string
  disabled?: boolean
  /** 该项对应的异步操作进行中：触发按钮转圈，且该项置灰 */
  loading?: boolean
  danger?: boolean
  /** 在此项之前插入分隔线 */
  separatorBefore?: boolean
  command: () => void
}

const props = withDefaults(
  defineProps<{
    items: FpActionItem[]
    ariaLabel?: string
  }>(),
  { ariaLabel: '' },
)

const { t } = useI18n()
/** 有 Pinia 时经 store 取角色（响应式）；无 Pinia（单测/极端场景）回落 resolveRole 的 fail-closed 语义 */
const auth = getActivePinia() ? useAuthStore() : null
const menuRef = ref<InstanceType<typeof TieredMenu>>()
const open = ref(false)

const visibleItems = computed(() =>
  props.items.filter(
    (it) => !it.perm || hasPermission(auth ? auth.role || '' : resolveRole(), it.perm),
  ),
)

const anyLoading = computed(() => visibleItems.value.some((it) => it.loading))

const menuModel = computed<MenuItem[]>(() => {
  const out: MenuItem[] = []
  visibleItems.value.forEach((it, idx) => {
    const prevDanger = idx > 0 && !!visibleItems.value[idx - 1].danger
    if (idx > 0 && (it.separatorBefore || (it.danger && !prevDanger))) out.push({ separator: true })
    out.push({
      key: it.key,
      label: it.label,
      icon: it.icon,
      disabled: it.disabled || it.loading,
      danger: it.danger,
      command: () => it.command(),
    })
  })
  return out
})

function toggle(e: Event) {
  menuRef.value?.toggle(e)
}
</script>

<style>
/* TieredMenu 经 append-to=body 挂载，需全局样式（fp-action-menu 前缀隔离） */
.fp-action-menu.p-tieredmenu {
  min-width: 160px;
}
.fp-action-menu .fp-action-menu__item {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
  padding: 7px var(--fp-space-3);
  font-size: 13px;
  color: var(--fp-text-primary);
  cursor: pointer;
  text-decoration: none;
}
.fp-action-menu .fp-action-menu__item:hover {
  background: var(--fp-bg-hover, var(--fp-bg-subtle));
}
.fp-action-menu .fp-action-menu__icon {
  font-size: 13px;
  width: 16px;
  text-align: center;
  color: var(--fp-text-muted);
}
.fp-action-menu .fp-action-menu__item--danger,
.fp-action-menu .fp-action-menu__item--danger .fp-action-menu__icon {
  color: var(--fp-danger);
}
.fp-action-menu .p-disabled {
  opacity: 0.45;
  pointer-events: none;
}
</style>
