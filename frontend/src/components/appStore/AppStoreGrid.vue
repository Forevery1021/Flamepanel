<template>
  <div>
    <!-- 推荐安装（仅商店页签；后端 recommended 标记） -->
    <div v-if="showRecommend && recommended.length" class="recommend-section">
      <div class="section-title">{{ t('appStore.recommended') }}</div>
      <div class="recommend-list">
        <div
          v-for="pkg in recommended"
          :key="pkg.key"
          class="recommend-card"
          @click="emit('install', pkg)"
        >
          <div class="recommend-logo"><i class="oi oi-box" /></div>
          <div class="recommend-info">
            <div class="recommend-name">{{ pkg.name }}</div>
            <div class="recommend-desc">{{ localizedDesc(pkg) }}</div>
            <FpTag severity="warning" :value="t('appStore.recommended')" />
          </div>
          <FpButton
            v-permission="{ perm: 'app_store:create', mode: 'view' }"
            variant="primary"
            @click.stop="emit('install', pkg)"
          >
            {{ installedKeys.has(pkg.key) ? t('appStore.installed') : t('appStore.install') }}
          </FpButton>
        </div>
      </div>
    </div>

    <div v-if="loading" class="app-grid">
      <div v-for="i in 8" :key="i" class="app-card app-card-skeleton">
        <FpSkeleton height="44px" />
        <FpSkeleton height="16px" />
        <FpSkeleton height="12px" />
      </div>
    </div>
    <div v-else-if="packages.length" class="app-grid">
      <div v-for="pkg in packages" :key="pkg.key" class="app-card">
        <div class="app-card-body">
          <div class="app-logo" :class="{ 'wasm-logo': wasm || pkg.category === 'wasm' }">
            <i :class="wasm ? 'oi oi-sparkles' : 'oi oi-box'" />
          </div>
          <div class="app-info">
            <div class="app-name">{{ pkg.name }}</div>
            <div class="app-desc">{{ localizedDesc(pkg) }}</div>
            <div v-if="!wasm" class="app-tags">
              <FpTag severity="info" :value="formatLabel(pkg.format)" />
              <FpTag v-for="tag in pkg.tags" :key="tag" :value="tag" />
            </div>
          </div>
        </div>
        <div class="app-actions">
          <FpButton :variant="buttonVariant(pkg.key)" @click="emit('install', pkg)">
            {{ buttonLabel(pkg.key) }}
          </FpButton>
        </div>
      </div>
    </div>
    <FpEmpty v-else :description="t('appStore.empty')" icon="oi oi-inbox" />
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useLocalizedDesc } from '@/composables/useLocalizedDesc'
import { formatLabel } from '@/utils/appStoreFormat'
import type { AppMetadata } from '@/api/appStore'
import FpButton from '@/components/ui/FpButton.vue'
import FpTag from '@/components/ui/FpTag.vue'
import FpEmpty from '@/components/ui/FpEmpty.vue'
import FpSkeleton from '@/components/ui/FpSkeleton.vue'

/**
 * 应用卡片网格（Doc/23 §3.3 T2-4 视图拆分）。
 *
 * 同一份「卡片 + 骨架屏 + 空态」供商店页签与 WASM 页签复用——两处的差异只有
 * 三处（页签图标 / 是否展示格式标签 / 安装按钮的已安装态），用 `wasm` 布尔收口，
 * 避免过去两份近似模板各自演化。
 *
 * **纯展示组件**：数据与操作全部经 props / emits 传递（数据逻辑在
 * `useAppStoreCatalog`），便于单测与后续复用。
 */
const props = withDefaults(
  defineProps<{
    /** 已过滤的包列表 */
    packages: AppMetadata[]
    /** 推荐位（仅商店页签传入） */
    recommended?: AppMetadata[]
    /** 已安装包 key 集合（按钮角标状态） */
    installedKeys?: Set<string>
    loading?: boolean
    /** WASM 内置工具页签：图标 / 标签 / 按钮语义按该模式渲染 */
    wasm?: boolean
    /** 是否展示推荐区（默认展示；无推荐数据时自然不渲染） */
    showRecommend?: boolean
  }>(),
  {
    recommended: () => [],
    installedKeys: () => new Set<string>(),
    loading: false,
    wasm: false,
    showRecommend: true,
  },
)

const emit = defineEmits<{ install: [pkg: AppMetadata] }>()

const { t } = useI18n()
const { localizedDesc } = useLocalizedDesc()

/** 已安装 → 成功色（角标语义）；WASM 无「已安装」状态，恒主色 */
function buttonVariant(key: string): 'success' | 'primary' {
  if (props.wasm) return 'primary'
  return props.installedKeys.has(key) ? 'success' : 'primary'
}

function buttonLabel(key: string): string {
  if (props.wasm) return t('appStore.install')
  return props.installedKeys.has(key) ? t('appStore.installed') : t('appStore.install')
}
</script>

<style scoped>
.section-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--fp-text-primary);
  margin-bottom: var(--fp-space-3);
}
.recommend-section {
  margin-bottom: var(--fp-space-4);
}
.recommend-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: var(--fp-space-3);
}
.recommend-card {
  display: flex;
  align-items: center;
  gap: var(--fp-space-3);
  padding: var(--fp-space-4);
  border-radius: var(--fp-radius-lg);
  border: 1px solid var(--fp-border);
  background: var(--fp-bg-surface);
  cursor: pointer;
  transition:
    border-color var(--fp-transition-fast),
    box-shadow var(--fp-transition-fast);
}
.recommend-card:hover {
  border-color: var(--fp-brand);
  box-shadow: var(--fp-shadow-lg);
}
.recommend-logo {
  width: 44px;
  height: 44px;
  border-radius: var(--fp-radius-md);
  background: var(--fp-brand-soft);
  color: var(--fp-brand);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 22px;
  flex-shrink: 0;
}
.recommend-info {
  flex: 1;
  min-width: 0;
}
.recommend-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--fp-text-primary);
}
.recommend-desc {
  font-size: 12px;
  color: var(--fp-text-secondary);
  margin: 2px 0 6px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.app-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: var(--fp-space-3);
}
.app-card {
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  gap: var(--fp-space-3);
  padding: var(--fp-space-4);
  border-radius: var(--fp-radius-md);
  background: var(--fp-bg-surface);
  border: 1px solid var(--fp-border);
  transition:
    box-shadow var(--fp-transition-fast),
    transform 120ms var(--fp-ease-out);
}
.app-card:hover {
  box-shadow: var(--fp-shadow-lg);
  transform: translateY(-1px);
}
.app-card-skeleton {
  gap: var(--fp-space-3);
}
.app-card-body {
  display: flex;
  gap: var(--fp-space-3);
  align-items: flex-start;
}
.app-logo {
  width: 44px;
  height: 44px;
  border-radius: var(--fp-radius-md);
  background: var(--fp-brand-soft);
  color: var(--fp-brand);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 22px;
  flex-shrink: 0;
}
.wasm-logo {
  background: var(--fp-info-soft);
  color: var(--fp-info);
}
.app-info {
  flex: 1;
  min-width: 0;
}
.app-name {
  font-weight: 600;
  color: var(--fp-text-primary);
  margin-bottom: 4px;
}
.app-desc {
  font-size: 12px;
  color: var(--fp-text-secondary);
  margin-bottom: var(--fp-space-2);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.app-tags {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fp-space-1);
}
.app-actions {
  display: flex;
  justify-content: flex-end;
}
</style>
