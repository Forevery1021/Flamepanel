<template>
  <LayoutContent :title="t('nav.settings')" :description="t('pageDesc.settings')">
    <div class="settings-shell">
      <!-- 二级导航：≥1100px 左侧竖排（sticky），更窄时变为可横向滚动的分段条 -->
      <nav
        class="settings-nav fp-surface"
        :aria-label="t('settings.title')"
        role="tablist"
        aria-orientation="vertical"
      >
        <button
          v-for="s in sections"
          :id="`settings-tab-${s.id}`"
          :key="s.id"
          type="button"
          role="tab"
          class="settings-nav__item"
          :class="{ 'is-active': active === s.id }"
          :aria-selected="active === s.id"
          :aria-controls="`settings-panel-${s.id}`"
          :tabindex="active === s.id ? 0 : -1"
          @click="go(s.id)"
          @keydown="onNavKeydown"
        >
          <i class="oi settings-nav__icon" :class="s.icon" aria-hidden="true" />
          <span class="settings-nav__label">{{ s.label }}</span>
          <span
            v-if="dirtyBySection[s.id]"
            class="settings-nav__dot"
            :title="t('settings.unsaved')"
            aria-hidden="true"
          />
        </button>
      </nav>

      <section
        :id="`settings-panel-${active}`"
        class="settings-panel"
        role="tabpanel"
        :aria-labelledby="`settings-tab-${active}`"
      >
        <header class="settings-panel__head">
          <h2 class="settings-panel__title">{{ activeSection.label }}</h2>
          <p class="settings-panel__desc">{{ activeSection.desc }}</p>
        </header>

        <!-- ── 面板 ── -->
        <template v-if="active === 'general'">
          <div class="settings-group fp-surface">
            <h3 class="settings-group__title">{{ t('settings.groupBasic') }}</h3>
            <SettingRow
              v-slot="{ id }"
              :label="t('settings.panelName')"
              :description="t('settings.panelNameHint')"
            >
              <FpInput v-model="generalForm.panel_name" :input-id="id" maxlength="64" />
            </SettingRow>
            <SettingRow v-slot="{ id }" :label="t('settings.language')">
              <FpSelect
                v-model="generalForm.language"
                :input-id="id"
                :options="langOptions"
                option-label="label"
                option-value="value"
                @update:model-value="(v) => setLanguage(String(v ?? 'zh-CN'))"
              />
            </SettingRow>
            <SettingRow
              v-slot="{ id }"
              :label="t('settings.serverAddress')"
              :description="t('settings.serverAddressHint')"
            >
              <FpInput
                v-model="generalForm.server_address"
                :input-id="id"
                maxlength="255"
                placeholder="https://panel.example.com"
              />
            </SettingRow>
            <SettingRow
              :label="t('settings.panelPort')"
              :badge="t('settings.readOnly')"
              :description="t('settings.panelPortHint')"
            >
              <code class="settings-code">{{ settingsMap.panel_port || '8080' }}</code>
            </SettingRow>
          </div>

          <div class="settings-group fp-surface">
            <h3 class="settings-group__title">{{ t('settings.groupLogs') }}</h3>
            <SettingRow
              v-slot="{ id }"
              :label="t('settings.logLevel')"
              :description="t('settings.logLevelHint')"
            >
              <FpSelect
                v-model="generalForm.log_level"
                :input-id="id"
                :options="logLevelOptions"
                option-label="label"
                option-value="value"
              />
            </SettingRow>
            <SettingRow
              v-slot="{ id }"
              :label="t('settings.logRetention')"
              :description="t('settings.logRetentionHint')"
            >
              <FpNumber
                v-model="generalForm.log_retention_days"
                :input-id="id"
                :min="1"
                :max="3650"
                :suffix="` ${t('settings.daysUnit')}`"
              />
            </SettingRow>
            <SettingRow
              v-slot="{ id }"
              :label="t('settings.auditRetention')"
              :description="t('settings.auditRetentionHint')"
            >
              <FpNumber
                v-model="generalForm.audit_log_retention_days"
                :input-id="id"
                :min="7"
                :max="3650"
                :suffix="` ${t('settings.daysUnit')}`"
              />
            </SettingRow>
          </div>

          <SaveBar
            :dirty="!!dirtyBySection.general"
            :saving="saving === 'general'"
            @save="save('general')"
            @reset="reset"
          />
        </template>

        <!-- ── 安全 ── -->
        <template v-else-if="active === 'security'">
          <div class="settings-group fp-surface">
            <h3 class="settings-group__title">{{ t('settings.groupAccess') }}</h3>
            <div v-if="access" class="access-card" :class="{ 'is-local': access.local }">
              <i class="oi oi-info-circle" aria-hidden="true" />
              <i18n-t keypath="settings.accessCurrent" tag="p" class="access-card__text">
                <template #ip>
                  <code>{{ access.client_ip || '—' }}</code>
                </template>
                <template #host>
                  <code>{{ access.host || '—' }}</code>
                </template>
              </i18n-t>
              <span v-if="access.local" class="access-card__local">{{
                t('settings.accessLocal')
              }}</span>
            </div>
            <SettingRow
              v-slot="{ id }"
              block
              :label="t('settings.allowedIps')"
              :description="t('settings.allowedIpsHint')"
            >
              <FpTextarea
                v-model="securityForm.allowed_ips"
                :input-id="id"
                rows="3"
                auto-resize
                class="mono-input"
                placeholder="192.168.1.0/24&#10;203.0.113.7"
              />
            </SettingRow>
            <SettingRow
              v-slot="{ id }"
              block
              :label="t('settings.bindDomains')"
              :description="t('settings.bindDomainsHint')"
            >
              <FpTextarea
                v-model="securityForm.bind_domains"
                :input-id="id"
                rows="2"
                auto-resize
                class="mono-input"
                placeholder="panel.example.com"
              />
            </SettingRow>
          </div>

          <div class="settings-group fp-surface">
            <h3 class="settings-group__title">{{ t('settings.groupLogin') }}</h3>
            <SettingRow
              v-slot="{ id }"
              :label="t('settings.sessionTimeout')"
              :description="t('settings.sessionTimeoutHint')"
            >
              <FpNumber
                v-model="securityForm.session_timeout_minutes"
                :input-id="id"
                :min="5"
                :max="43200"
                :suffix="` ${t('settings.minutesUnit')}`"
              />
            </SettingRow>
            <SettingRow
              v-slot="{ id }"
              :label="t('settings.loginMaxFailures')"
              :description="t('settings.loginMaxFailuresHint')"
            >
              <FpNumber
                v-model="securityForm.login_max_failures"
                :input-id="id"
                :min="3"
                :max="20"
              />
            </SettingRow>
            <SettingRow
              v-slot="{ id }"
              :label="t('settings.loginLockout')"
              :description="t('settings.loginLockoutHint')"
            >
              <FpNumber
                v-model="securityForm.login_lockout_minutes"
                :input-id="id"
                :min="1"
                :max="1440"
                :suffix="` ${t('settings.minutesUnit')}`"
              />
            </SettingRow>
          </div>

          <SaveBar
            :dirty="!!dirtyBySection.security"
            :saving="saving === 'security'"
            @save="save('security')"
            @reset="reset"
          />

          <SettingsCredentialsPanel />
        </template>

        <!-- ── 备份 ── -->
        <template v-else-if="active === 'backup'">
          <div class="settings-group fp-surface">
            <SettingRow
              v-slot="{ id }"
              :label="t('settings.autoBackup')"
              :description="t('settings.autoBackupHint')"
            >
              <FpSwitch v-model="backupForm.auto_backup_enabled" :input-id="id" />
            </SettingRow>
            <SettingRow v-slot="{ id }" :label="t('settings.backupInterval')">
              <FpNumber
                v-model="backupForm.auto_backup_interval_hours"
                :input-id="id"
                :min="1"
                :max="720"
                :disabled="!backupForm.auto_backup_enabled"
                :suffix="` ${t('settings.hoursUnit')}`"
              />
            </SettingRow>
            <SettingRow
              v-slot="{ id }"
              :label="t('settings.backupRetention')"
              :description="t('settings.backupRetentionHint')"
            >
              <FpNumber
                v-model="backupForm.backup_retention"
                :input-id="id"
                :min="1"
                :max="365"
                :disabled="!backupForm.auto_backup_enabled"
              />
            </SettingRow>
          </div>
          <SaveBar
            :dirty="!!dirtyBySection.backup"
            :saving="saving === 'backup'"
            @save="save('backup')"
            @reset="reset"
          />
        </template>

        <!-- ── DNS 厂商账号 ── -->
        <SettingsDnsTab v-else-if="active === 'dns'" />

        <!-- ── 主题 ── -->
        <SettingsThemeTab v-else-if="active === 'theme'" />

        <!-- ── 界面 ── -->
        <SettingsAppearanceTab v-else-if="active === 'appearance'" />

        <!-- ── 关于 ── -->
        <SettingsAboutTab v-else-if="active === 'about'" />
      </section>
    </div>
  </LayoutContent>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, computed, watch, defineComponent, h } from 'vue'
import { onBeforeRouteLeave, useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'

import type { AccessInfo } from '@/api/generated'
import { getAccessInfo, listSettings, updateSettingsBatch } from '@/api/settings'
import { setLanguage } from '@/locales'
import { useThemeStore } from '@/stores/theme'
import { useAppearanceStore } from '@/stores/appearance'
import LayoutContent from '@/components/ui/LayoutContent.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpNumber from '@/components/ui/FpNumber.vue'
import FpTextarea from '@/components/ui/FpTextarea.vue'
import FpSwitch from '@/components/ui/FpSwitch.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import SettingRow from '@/components/settings/SettingRow.vue'
import SettingsThemeTab from '@/components/settings/SettingsThemeTab.vue'
import SettingsDnsTab from '@/components/settings/SettingsDnsTab.vue'
import SettingsAppearanceTab from '@/components/settings/SettingsAppearanceTab.vue'
import SettingsAboutTab from '@/components/settings/SettingsAboutTab.vue'
import SettingsCredentialsPanel from '@/components/settings/SettingsCredentialsPanel.vue'

/**
 * 面板设置（W4-a 重写，参照 1Panel「面板设置」/ 宝塔「面板设置」的分组）。
 *
 * - **行式布局**：左侧「标题 + 说明」、右侧紧凑控件（`SettingRow`），替代旧版两列表单网格，
 *   控件不再撑满、按钮默认 small；容器 < 560px 自动堆叠。
 * - **分区即表单**：面板 / 安全 / 备份三个分区各有独立的脏状态与底部吸附保存条，
 *   保存走 `PATCH /api/settings/batch` 一次事务、只发送变更键（与服务端快照 diff）。
 * - **安全分区**新增访问控制（IP 白名单 / 绑定域名，保存前服务端校验「不会把自己锁在门外」）、
 *   登录失败锁定与会话时长；JWT 轮换改走 `POST /api/auth/rotate-secret`（旧版把随机串写进
 *   设置表，既没有真正轮换，还让只读账号看得到）。
 * - 端口只读展示：改端口需同时改防火墙 / 反代，放在 `fpctl` 里做更安全。
 */
const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const toast = useFpToast()
const { confirmAction } = useFpConfirm()
const themeStore = useThemeStore()
const appearance = useAppearanceStore()

/** 分区底部保存条：吸附在视口底部，只有改动时高亮 */
const SaveBar = defineComponent({
  name: 'SettingsSaveBar',
  props: { dirty: Boolean, saving: Boolean },
  emits: ['save', 'reset'],
  setup(props, { emit }) {
    return () =>
      h('div', { class: ['settings-actions', { 'is-dirty': props.dirty }] }, [
        h(
          FpButton,
          { loading: props.saving, disabled: !props.dirty, onClick: () => emit('save') },
          () => t('settings.save'),
        ),
        h(
          FpButton,
          { variant: 'ghost', text: true, disabled: !props.dirty, onClick: () => emit('reset') },
          () => t('common.reset'),
        ),
        props.dirty
          ? h('span', { class: 'settings-actions__hint' }, t('settings.unsavedHint'))
          : null,
      ])
  },
})

// ── 分区导航 ──
type SectionId = 'general' | 'security' | 'backup' | 'dns' | 'theme' | 'appearance' | 'about'
type FormSection = 'general' | 'security' | 'backup'
const SECTION_DEFS: Array<{ id: SectionId; icon: string; labelKey: string }> = [
  { id: 'general', icon: 'oi-sliders-h', labelKey: 'settings.panelConfig' },
  { id: 'security', icon: 'oi-shield', labelKey: 'settings.security' },
  { id: 'backup', icon: 'oi-database', labelKey: 'settings.backupSettings' },
  { id: 'dns', icon: 'oi-globe', labelKey: 'dnsProvider.title' },
  { id: 'theme', icon: 'oi-palette', labelKey: 'settingsTheme.title' },
  { id: 'appearance', icon: 'oi-desktop', labelKey: 'settingsAppearance.title' },
  { id: 'about', icon: 'oi-info-circle', labelKey: 'settings.about' },
]
const sections = computed(() =>
  SECTION_DEFS.map((s) => ({
    ...s,
    label: t(s.labelKey),
    desc: t(`settings.sectionDesc.${s.id}`),
  })),
)
const SECTION_IDS = new Set<string>(SECTION_DEFS.map((s) => s.id))

function sectionFromHash(hash: string): SectionId {
  const id = hash.replace(/^#/, '')
  return (SECTION_IDS.has(id) ? id : 'general') as SectionId
}

const active = ref<SectionId>(sectionFromHash(route.hash))
const activeSection = computed(
  () => sections.value.find((s) => s.id === active.value) ?? sections.value[0],
)

function go(id: SectionId) {
  if (active.value === id) return
  active.value = id
  router.replace({ hash: `#${id}` }).catch(() => {})
}
watch(
  () => route.hash,
  (hash) => {
    const id = sectionFromHash(hash)
    if (id !== active.value) active.value = id
  },
)
/** 窄屏横向分段条：把当前分区滚进可视区（深链到靠后的分区时不至于看不到高亮） */
function revealTab(id: SectionId) {
  const el = document.getElementById(`settings-tab-${id}`)
  el?.scrollIntoView?.({ block: 'nearest', inline: 'nearest' })
}
watch(active, (id) => {
  revealTab(id)
  if (id === 'security') loadAccess()
})

/** 二级导航键盘操作：↑↓ / ←→ 切换，Home/End 到首尾（WAI-ARIA tablist） */
function onNavKeydown(e: KeyboardEvent) {
  const ids = SECTION_DEFS.map((s) => s.id)
  const i = ids.indexOf(active.value)
  let next = i
  if (e.key === 'ArrowDown' || e.key === 'ArrowRight') next = (i + 1) % ids.length
  else if (e.key === 'ArrowUp' || e.key === 'ArrowLeft') next = (i - 1 + ids.length) % ids.length
  else if (e.key === 'Home') next = 0
  else if (e.key === 'End') next = ids.length - 1
  else return
  e.preventDefault()
  go(ids[next])
  document.getElementById(`settings-tab-${ids[next]}`)?.focus()
}

// ── 选项 ──
const langOptions = [
  { label: '简体中文', value: 'zh-CN' },
  { label: 'English', value: 'en-US' },
  { label: '日本語', value: 'ja-JP' },
]
const logLevelOptions = ['trace', 'debug', 'info', 'warn', 'error'].map((v) => ({
  label: v.toUpperCase(),
  value: v,
}))

// ── 表单模型 ──
const settingsMap = ref<Record<string, string>>({})
const DEFAULTS: Record<string, string> = {
  panel_name: 'FlamePanel',
  language: 'zh-CN',
  server_address: '',
  log_level: 'info',
  log_retention_days: '30',
  audit_log_retention_days: '90',
  allowed_ips: '',
  bind_domains: '',
  session_timeout_minutes: '1440',
  login_max_failures: '5',
  login_lockout_minutes: '5',
  auto_backup_enabled: 'false',
  auto_backup_interval_hours: '24',
  backup_retention: '7',
}

const generalForm = reactive({
  panel_name: '',
  language: 'zh-CN',
  server_address: '',
  log_level: 'info',
  log_retention_days: 30 as number | null,
  audit_log_retention_days: 90 as number | null,
})
const securityForm = reactive({
  allowed_ips: '',
  bind_domains: '',
  session_timeout_minutes: 1440 as number | null,
  login_max_failures: 5 as number | null,
  login_lockout_minutes: 5 as number | null,
})
const backupForm = reactive({
  auto_backup_enabled: false,
  auto_backup_interval_hours: 24 as number | null,
  backup_retention: 7 as number | null,
})

/** 列表类设置的规范形：分隔符统一为逗号（与服务端 `settings_schema` 的归一化一致） */
function canonList(v: string, lower = false): string {
  const items = v
    .split(/[\s,;]+/)
    .map((s) => s.trim())
    .filter(Boolean)
  return (lower ? items.map((s) => s.toLowerCase()) : items).join(',')
}
const listToLines = (v: string) => canonList(v).split(',').filter(Boolean).join('\n')
const num = (v: number | null, fallback: string) => (v == null ? fallback : String(v))

/** 表单 → 服务端键值（diff / 保存 / 重置共用的单一映射） */
function pairsOf(section: FormSection): Record<string, string> {
  if (section === 'general') {
    return {
      panel_name: generalForm.panel_name.trim(),
      language: generalForm.language,
      server_address: generalForm.server_address.trim(),
      log_level: generalForm.log_level,
      log_retention_days: num(generalForm.log_retention_days, DEFAULTS.log_retention_days),
      audit_log_retention_days: num(
        generalForm.audit_log_retention_days,
        DEFAULTS.audit_log_retention_days,
      ),
    }
  }
  if (section === 'security') {
    return {
      allowed_ips: canonList(securityForm.allowed_ips),
      bind_domains: canonList(securityForm.bind_domains, true),
      session_timeout_minutes: num(
        securityForm.session_timeout_minutes,
        DEFAULTS.session_timeout_minutes,
      ),
      login_max_failures: num(securityForm.login_max_failures, DEFAULTS.login_max_failures),
      login_lockout_minutes: num(
        securityForm.login_lockout_minutes,
        DEFAULTS.login_lockout_minutes,
      ),
    }
  }
  return {
    auto_backup_enabled: backupForm.auto_backup_enabled ? 'true' : 'false',
    auto_backup_interval_hours: num(
      backupForm.auto_backup_interval_hours,
      DEFAULTS.auto_backup_interval_hours,
    ),
    backup_retention: num(backupForm.backup_retention, DEFAULTS.backup_retention),
  }
}

function serverValue(key: string): string {
  const raw = settingsMap.value[key] ?? DEFAULTS[key] ?? ''
  if (key === 'allowed_ips') return canonList(raw)
  if (key === 'bind_domains') return canonList(raw, true)
  return raw
}

/** 只返回与服务端快照不同的键 */
function changedPairs(section: FormSection): [string, string][] {
  return Object.entries(pairsOf(section)).filter(([k, v]) => serverValue(k) !== v)
}

const dirtyBySection = computed<Partial<Record<SectionId, boolean>>>(() => ({
  general: changedPairs('general').length > 0,
  security: changedPairs('security').length > 0,
  backup: changedPairs('backup').length > 0,
}))
const anyDirty = computed(() => Object.values(dirtyBySection.value).some(Boolean))

function applyMapToForms() {
  const v = (k: string) => settingsMap.value[k] || DEFAULTS[k]
  const n = (k: string) => {
    const parsed = parseInt(v(k), 10)
    return Number.isFinite(parsed) ? parsed : parseInt(DEFAULTS[k], 10)
  }
  generalForm.panel_name = v('panel_name')
  generalForm.language = v('language')
  generalForm.server_address = settingsMap.value.server_address ?? ''
  generalForm.log_level = v('log_level')
  generalForm.log_retention_days = n('log_retention_days')
  generalForm.audit_log_retention_days = n('audit_log_retention_days')
  securityForm.allowed_ips = listToLines(settingsMap.value.allowed_ips ?? '')
  securityForm.bind_domains = listToLines(settingsMap.value.bind_domains ?? '')
  securityForm.session_timeout_minutes = n('session_timeout_minutes')
  securityForm.login_max_failures = n('login_max_failures')
  securityForm.login_lockout_minutes = n('login_lockout_minutes')
  backupForm.auto_backup_enabled = settingsMap.value.auto_backup_enabled === 'true'
  backupForm.auto_backup_interval_hours = n('auto_backup_interval_hours')
  backupForm.backup_retention = n('backup_retention')
}

// ── 读取 / 保存 ──
const loading = ref(false)
const saving = ref<FormSection | ''>('')

async function fetchSettings() {
  loading.value = true
  try {
    const res = await listSettings(1, 200)
    const map: Record<string, string> = {}
    for (const s of res.data.data) map[s.key] = s.value
    settingsMap.value = map
    appearance.syncFromServer(map)
    themeStore.syncFromServer(map)
    applyMapToForms()
  } catch (e: unknown) {
    toast.error(e, t('common.failed'))
  } finally {
    loading.value = false
  }
}

async function save(section: FormSection) {
  const pairs = changedPairs(section)
  if (pairs.length === 0) {
    toast.info(t('settings.nothingToSave'))
    return
  }
  saving.value = section
  try {
    await updateSettingsBatch(pairs)
    const next = { ...settingsMap.value }
    for (const [k, val] of pairs) next[k] = val
    settingsMap.value = next
    toast.success(t('common.success'))
    if (section === 'security') loadAccess()
  } catch (e: unknown) {
    // 服务端校验信息（越界 / 会锁住当前 IP）原样透出，用户能看到具体原因
    toast.error(e, t('common.failed'))
  } finally {
    saving.value = ''
  }
}

function reset() {
  applyMapToForms()
}

// ── 安全：访问信息（改密 / 2FA / 轮换见 SettingsCredentialsPanel） ──
const access = ref<AccessInfo | null>(null)
async function loadAccess() {
  try {
    access.value = (await getAccessInfo()).data
  } catch {
    access.value = null
  }
}

// 离开页面前：有未保存改动 → 二次确认（同页 hash 切换不拦截）
onBeforeRouteLeave((to, from) => {
  if (!anyDirty.value || to.path === from.path) return true
  return new Promise<boolean>((resolve) => {
    confirmAction({
      header: t('settings.unsaved'),
      message: t('settings.leaveConfirm'),
      acceptLabel: t('settings.discard'),
      rejectLabel: t('common.cancel'),
      accept: () => resolve(true),
      reject: () => resolve(false),
    })
  })
})

onMounted(() => {
  fetchSettings()
  revealTab(active.value)
  if (active.value === 'security') loadAccess()
})
</script>

<style scoped>
.settings-shell {
  display: grid;
  grid-template-columns: 200px minmax(0, 1fr);
  gap: var(--fp-space-5);
  align-items: start;
}

/* ── 二级导航 ── */
.settings-nav {
  position: sticky;
  top: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: var(--fp-space-2);
  border: 1px solid var(--fp-border);
  border-radius: var(--fp-radius-lg);
}
.settings-nav__item {
  position: relative;
  display: flex;
  align-items: center;
  gap: 10px;
  height: 34px;
  padding: 0 10px;
  border: 0;
  border-radius: var(--fp-radius-sm);
  background: transparent;
  color: var(--fp-text-secondary);
  font: inherit;
  font-size: var(--fp-font-sm);
  text-align: left;
  white-space: nowrap;
  cursor: pointer;
  transition:
    background-color var(--fp-transition-fast),
    color var(--fp-transition-fast);
}
.settings-nav__item:hover {
  background: var(--fp-bg-hover);
  color: var(--fp-text-primary);
}
.settings-nav__item.is-active {
  background: var(--fp-brand-soft);
  color: var(--fp-brand);
  font-weight: 600;
}
.settings-nav__icon {
  width: 16px;
  font-size: 14px;
  text-align: center;
}
.settings-nav__label {
  flex: 1 1 auto;
  overflow: hidden;
  text-overflow: ellipsis;
}
.settings-nav__dot {
  width: 6px;
  height: 6px;
  border-radius: 999px;
  background: var(--fp-warning);
  flex: 0 0 auto;
}

/* ── 内容区（容器查询宿主：SettingRow 按内容宽度而不是视口宽度堆叠） ── */
.settings-panel {
  container: settings / inline-size;
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-4);
  min-width: 0;
  max-width: 920px;
}
.settings-panel__head {
  padding: 0 2px;
}
.settings-panel__title {
  margin: 0;
  font-size: var(--fp-font-lg);
  font-weight: 650;
  color: var(--fp-text-primary);
}
.settings-panel__desc {
  margin: 2px 0 0;
  font-size: var(--fp-font-sm);
  color: var(--fp-text-muted);
}

.settings-code {
  padding: 2px 8px;
  border-radius: var(--fp-radius-sm);
  background: var(--fp-bg-subtle);
  border: 1px solid var(--fp-border);
  font-family: var(--fp-font-mono);
  font-size: var(--fp-font-xs);
  color: var(--fp-text-primary);
}
.mono-input :deep(textarea) {
  font-family: var(--fp-font-mono);
  font-size: var(--fp-font-xs);
}

/* 访问来源提示条 */
.access-card {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: var(--fp-space-2);
  padding: 8px 12px;
  border-radius: var(--fp-radius-sm);
  background: var(--fp-info-soft);
  color: var(--fp-text-secondary);
  font-size: var(--fp-font-xs);
}
.access-card > .oi {
  color: var(--fp-info);
}
.access-card__text {
  margin: 0;
  flex: 1 1 auto;
}
.access-card__text code {
  font-family: var(--fp-font-mono);
  color: var(--fp-text-primary);
}
.access-card__local {
  padding: 0 8px;
  border-radius: 999px;
  line-height: 20px;
  background: var(--fp-success-soft);
  color: var(--fp-success);
}

/* ── 平板 / 窄窗口：导航变为顶部横向分段条 ── */
@media (max-width: 1100px) {
  .settings-shell {
    grid-template-columns: minmax(0, 1fr);
    gap: var(--fp-space-3);
  }
  .settings-nav {
    top: calc(-1 * var(--fp-space-3));
    z-index: 2;
    flex-direction: row;
    overflow-x: auto;
    scrollbar-width: none;
    padding: 4px;
  }
  .settings-nav::-webkit-scrollbar {
    display: none;
  }
  .settings-nav__item {
    flex: 0 0 auto;
    height: 30px;
  }
  .settings-panel {
    max-width: none;
  }
}
@media (max-width: 560px) {
  .settings-nav__icon {
    display: none;
  }
}
</style>

<!-- 设置分组 / 保存条：子组件（主题 / DNS）与渲染函数组件共用，不加 scoped -->
<style>
.settings-group,
.settings-card {
  padding: var(--fp-space-2) var(--fp-space-5);
  border: 1px solid var(--fp-border);
  border-radius: var(--fp-radius-lg);
}
.settings-card {
  padding: var(--fp-space-4) var(--fp-space-5);
}
.settings-group__title {
  margin: var(--fp-space-3) 0 var(--fp-space-1);
  font-size: var(--fp-font-xs);
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--fp-text-muted);
}
.settings-group__foot {
  display: flex;
  justify-content: flex-end;
  padding: var(--fp-space-2) 0 var(--fp-space-3);
}
.settings-actions {
  position: relative;
  bottom: 0;
  z-index: 1;
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: var(--fp-space-2);
  margin-top: calc(-1 * var(--fp-space-2));
  padding: var(--fp-space-2) var(--fp-space-3);
  border: 1px solid transparent;
  border-radius: var(--fp-radius-md);
  transition:
    background-color var(--fp-transition-fast),
    border-color var(--fp-transition-fast);
}
.settings-actions.is-dirty {
  /* 只有存在改动时才吸附在视口底部（未改动时吸附会遮住下方内容） */
  position: sticky;
  background: var(--fp-bg-elevated);
  border-color: var(--fp-border);
  box-shadow: var(--fp-shadow-md);
}
.settings-actions__hint {
  font-size: var(--fp-font-xs);
  color: var(--fp-warning);
}
@container settings (max-width: 560px) {
  .settings-group,
  .settings-card {
    padding: var(--fp-space-1) var(--fp-space-3);
  }
}
</style>
