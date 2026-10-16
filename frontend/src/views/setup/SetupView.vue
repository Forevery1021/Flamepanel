<template>
  <div class="setup">
    <div class="setup-bg" aria-hidden="true" />
    <div class="setup-glow" aria-hidden="true" />

    <div class="setup-container">
      <!-- 左侧品牌区 -->
      <div class="setup-brand" aria-hidden="true">
        <div class="setup-brand__inner">
          <span class="logo-mark">
            <svg viewBox="0 0 24 24" width="56" height="56" fill="none">
              <path
                d="M12 2c1 3.5-1 5.5-3 8-1.8 2.2-2.5 4-2 6.5A5.5 5.5 0 0 0 12 22a5.5 5.5 0 0 0 5-5.5c.5-2.5-.2-4.3-2-6.5-2-2.5-4-4.5-3-8Z"
                fill="url(#flameGrad)"
                opacity="0.92"
              />
              <circle cx="12" cy="16.5" r="2" fill="var(--fp-bg-elevated)" />
              <defs>
                <linearGradient id="flameGrad" x1="8" y1="2" x2="16" y2="22">
                  <stop offset="0%" stop-color="var(--fp-brand)" />
                  <stop offset="100%" stop-color="var(--fp-brand-strong)" />
                </linearGradient>
              </defs>
            </svg>
          </span>
          <h1 class="setup-brand__name">FlamePanel</h1>
          <p class="setup-brand__slogan">{{ t('setup.brandSlogan') }}</p>
          <ul class="setup-brand__features">
            <li><i class="oi oi-gauge" />{{ t('setup.featMonitor') }}</li>
            <li><i class="oi oi-database" />{{ t('setup.featDocker') }}</li>
            <li><i class="oi oi-shield" />{{ t('setup.featSecurity') }}</li>
          </ul>
        </div>
      </div>

      <!-- 右侧向导 -->
      <div class="setup-panel">
        <!-- 步骤指示器：已完成的步骤可点击回退修改；顶部进度条随步骤推进 -->
        <div class="setup-progress" aria-hidden="true">
          <span class="setup-progress__bar" :style="{ width: `${(current / (steps.length - 1)) * 100}%` }" />
        </div>
        <ol class="setup-steps" :aria-label="t('setup.stepsLabel')">
          <li
            v-for="(step, i) in steps"
            :key="step"
            class="setup-step"
            :class="{ active: i === current, done: i < current, clickable: i < current && !submitting }"
            :aria-current="i === current ? 'step' : undefined"
          >
            <button
              type="button"
              class="setup-step__btn"
              :disabled="i >= current || submitting"
              @click="jumpTo(i)"
            >
              <span class="setup-step__dot">{{ i < current ? '✓' : i + 1 }}</span>
              <span class="setup-step__label">{{ t(`setup.step.${step}`) }}</span>
            </button>
          </li>
        </ol>

        <p v-if="resumed" class="setup-resumed" role="status">
          <i class="oi oi-info-circle" aria-hidden="true" />
          {{ t('setup.resumed') }}
          <button type="button" class="setup-resumed__reset" @click="startOver">{{ t('setup.startOver') }}</button>
        </p>

        <div ref="bodyRef" class="setup-body" @keydown.enter="onEnter">
          <StepWelcome
            v-if="current === 0"
            :docker="status?.docker ?? null"
            :nginx="status?.nginx ?? null"
            @continue="goNext"
          />
          <StepAdmin
            v-else-if="current === 1"
            :username="form.admin.username"
            :password="form.admin.password"
            :confirm="form.admin.confirm"
            :errors="adminErrors"
            @update="onAdminUpdate"
          />
          <StepDatabase
            v-else-if="current === 2"
            :db="form.database"
            @update="onDatabaseUpdate"
          />
          <StepServer
            v-else-if="current === 3"
            :port="form.server.panel_port"
            :nginx="status?.nginx ?? null"
            @update="onServerUpdate"
          />
          <StepTheme
            v-else-if="current === 4"
            :theme="form.theme"
            :language="form.language"
            @update="onThemeUpdate"
          />
          <StepFinish
            v-else
            :summary="summary"
            :submitting="submitting"
            :error="finishError"
            @submit="handleSubmit"
          />
        </div>

        <div v-if="current > 0 && current < 5" class="setup-nav">
          <FpButton variant="secondary" :disabled="submitting" @click="current--">
            {{ t('setup.back') }}
          </FpButton>
          <span class="setup-nav__hint" aria-hidden="true">{{ t('setup.enterHint') }}</span>
          <FpButton variant="primary" :disabled="!canContinue || submitting" @click="goNext">
            {{ t('setup.next') }}
          </FpButton>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, reactive, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import FpButton from '@/components/ui/FpButton.vue'
import { STORAGE_KEYS } from '@/utils/storage'
import { useSetupStore } from '@/stores/setup'
import { useAuthStore } from '@/stores/auth'
import { useThemeStore, type ThemePreset } from '@/stores/theme'
import { setLanguage, type AppLocale } from '@/locales'
import StepWelcome from './StepWelcome.vue'
import StepAdmin from './StepAdmin.vue'
import StepDatabase from './StepDatabase.vue'
import StepServer from './StepServer.vue'
import StepTheme from './StepTheme.vue'
import StepFinish from './StepFinish.vue'

const { t } = useI18n()
const router = useRouter()
const setup = useSetupStore()
const auth = useAuthStore()
const theme = useThemeStore()

/**
 * 安装向导（UX-4 优化）。
 *
 * - **草稿续传**：非敏感字段（用户名 / 数据库类型与连接参数 / 端口 / 主题 / 语言）与当前步骤
 *   写入 `sessionStorage`（`STORAGE_KEYS.setupDraft`）；刷新或误关标签页后字段全部回填，
 *   **密码一律不落盘**，因此续传落点固定为「管理员」步（只需重输密码，其余步骤 Enter 直达）；
 * - **键盘流**：文本输入框内按 Enter = 下一步（可继续时），步骤切换后自动聚焦首个输入；
 * - **可回退的步骤条**：已完成的步骤可点击回到该步修改；`aria-current="step"` 标注当前步。
 */
const steps = ['welcome', 'admin', 'database', 'server', 'theme', 'finish'] as const
const current = ref(0)
const submitting = ref(false)
const finishError = ref('')
const status = ref(setup.status)
const resumed = ref(false)
const bodyRef = ref<HTMLElement | null>(null)

const form = reactive({
  admin: { username: 'admin', password: '', confirm: '' },
  database: {
    db_type: 'sqlite' as 'sqlite' | 'mysql' | 'mariadb',
    host: '127.0.0.1',
    port: 3306,
    name: 'flamepanel',
    user: 'flamepanel',
    password: '',
    mysql_root_password: '',
  },
  server: { panel_port: 8080 },
  theme: 'flame' as ThemePreset,
  language: 'zh-CN' as AppLocale,
})

const adminErrors = reactive({ username: '', password: '', confirm: '' })

const summary = computed(() => ({
  username: form.admin.username,
  dbType: form.database.db_type,
  dbName: form.database.db_type === 'sqlite' ? 'SQLite' : form.database.name,
  port: form.server.panel_port,
  theme: form.theme,
  language: form.language,
}))

/* ── 草稿续传（sessionStorage；密码字段不持久化） ── */
interface SetupDraft {
  v: 1
  step: number
  admin: { username: string }
  database: Omit<typeof form.database, 'password' | 'mysql_root_password'>
  server: typeof form.server
  theme: ThemePreset
  language: AppLocale
}

function saveDraft() {
  try {
    const { db_type, host, port, name, user } = form.database
    const draft: SetupDraft = {
      v: 1,
      step: current.value,
      admin: { username: form.admin.username },
      database: { db_type, host, port, name, user },
      server: { ...form.server },
      theme: form.theme,
      language: form.language,
    }
    sessionStorage.setItem(STORAGE_KEYS.setupDraft, JSON.stringify(draft))
  } catch {
    // 隐私模式等：不影响向导本身
  }
}

function loadDraft(): SetupDraft | null {
  try {
    const raw = sessionStorage.getItem(STORAGE_KEYS.setupDraft)
    if (!raw) return null
    const d = JSON.parse(raw) as Partial<SetupDraft>
    if (d.v !== 1 || typeof d.step !== 'number') return null
    return d as SetupDraft
  } catch {
    return null
  }
}

function clearDraft() {
  try {
    sessionStorage.removeItem(STORAGE_KEYS.setupDraft)
  } catch {
    // ignore
  }
}

function applyDraft(d: SetupDraft) {
  form.admin.username = d.admin?.username || form.admin.username
  Object.assign(form.database, d.database ?? {})
  Object.assign(form.server, d.server ?? {})
  if (d.theme) form.theme = d.theme
  if (d.language) form.language = d.language
  // 密码未持久化：续传统一落在「管理员」步补填密码（其余字段已回填，Enter 可连续下一步）
  current.value = Math.min(Math.max(d.step, 0), 1)
  resumed.value = d.step > 0
}

function startOver() {
  clearDraft()
  resumed.value = false
  form.admin.username = 'admin'
  form.admin.password = ''
  form.admin.confirm = ''
  form.database.db_type = 'sqlite'
  form.database.host = '127.0.0.1'
  form.database.port = 3306
  form.database.name = 'flamepanel'
  form.database.user = 'flamepanel'
  form.database.password = ''
  form.database.mysql_root_password = ''
  form.server.panel_port = 8080
  current.value = 0
}

watch(
  [current, () => form.admin.username, () => form.database, () => form.server, () => form.theme, () => form.language],
  () => {
    if (current.value > 0) saveDraft()
  },
  { deep: true },
)

/* ── 键盘流与焦点 ── */
function focusFirstField() {
  const el = bodyRef.value?.querySelector<HTMLElement>(
    'input:not([type="hidden"]):not([disabled]), [role="combobox"], button.step-cta',
  )
  el?.focus({ preventScroll: true })
}

watch(current, async () => {
  await nextTick()
  focusFirstField()
})

/** 文本输入框内 Enter → 下一步（下拉/按钮自身的 Enter 语义不劫持） */
function onEnter(e: KeyboardEvent) {
  const target = e.target as HTMLElement | null
  if (!target || target.tagName !== 'INPUT') return
  if (current.value === 0 || current.value >= steps.length - 1) return
  if (!canContinue.value || submitting.value) return
  e.preventDefault()
  goNext()
}

/** 步骤条点击：只允许回到已完成的步骤 */
function jumpTo(i: number) {
  if (i < current.value && !submitting.value) current.value = i
}

function validateAdmin(): boolean {
  adminErrors.username = form.admin.username.trim() ? '' : t('setup.usernameRequired')
  adminErrors.password =
    form.admin.password.length >= 8 ? '' : t('setup.passwordMinLength')
  adminErrors.confirm =
    form.admin.confirm && form.admin.confirm === form.admin.password
      ? ''
      : t('setup.passwordMismatch')
  return !adminErrors.username && !adminErrors.password && !adminErrors.confirm
}

const canContinue = computed(() => {
  switch (current.value) {
    case 1:
      return validateAdmin()
    case 2:
      return (
        form.database.db_type === 'sqlite' ||
        (!!form.database.name.trim() &&
          !!form.database.user.trim() &&
          !!form.database.password &&
          !!form.database.mysql_root_password)
      )
    case 3:
      return form.server.panel_port > 0 && form.server.panel_port <= 65535
    default:
      return true
  }
})

function goNext() {
  if (current.value === 1 && !validateAdmin()) return
  if (current.value < steps.length - 1) current.value++
}

function onAdminUpdate(patch: Partial<typeof form.admin>) {
  Object.assign(form.admin, patch)
  validateAdmin()
}

function onDatabaseUpdate(patch: Partial<typeof form.database>) {
  Object.assign(form.database, patch)
}

function onServerUpdate(patch: Partial<typeof form.server>) {
  Object.assign(form.server, patch)
}

function onThemeUpdate(patch: { theme?: ThemePreset; language?: AppLocale }) {
  if (patch.theme) {
    form.theme = patch.theme
    theme.setPreset(patch.theme)
    theme.apply()
  }
  if (patch.language) {
    form.language = patch.language
    void setLanguage(patch.language)
  }
}

async function handleSubmit() {
  submitting.value = true
  finishError.value = ''
  try {
    // 阶段 1：数据库配置（sqlite 本地直用；mysql 由后端建库建用户）
    await setup.complete({
      step: 'database',
      database: form.database,
    })
    // 阶段 2：管理员账号 + 主题/语言 + setup_completed_at（终态）
    const res = await setup.complete({
      step: 'admin',
      admin: { username: form.admin.username.trim(), password: form.admin.password },
      theme: form.theme,
      language: form.language,
    })
    clearDraft()
    if (res.token && res.username && res.role) {
      // 直接进入登录态（免二次登录）
      // Y1：refresh token 由后端经 httpOnly Cookie 下发，前端仅保存 access token 与身份信息
      auth.save({
        token: res.token,
        refresh_token: '',
        username: res.username,
        role: res.role,
        must_change_password: false,
      })
    }
    router.push('/dashboard')
  } catch (e) {
    const err = e as { code?: string; message?: string; status?: number }
    finishError.value =
      err.status === 409
        ? t('setup.alreadyInitialized')
        : (err.message || t('setup.submitFailed'))
    current.value = 5
  } finally {
    submitting.value = false
  }
}

onMounted(async () => {
  const s = await setup.ensureStatus()
  status.value = s
  if (s) {
    form.theme = (s.theme as ThemePreset) || 'flame'
    form.language = (s.language as AppLocale) || 'zh-CN'
  }
  const draft = loadDraft()
  if (draft) applyDraft(draft)
  theme.setPreset(form.theme)
  theme.apply()
  await nextTick()
  focusFirstField()
})
</script>

<style scoped>
.setup {
  position: relative;
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--fp-space-4);
  background: var(--fp-bg-app);
  overflow: hidden;
}
.setup-bg {
  position: absolute;
  inset: 0;
  background: var(--fp-login-bg);
  background-size: cover;
  background-position: center;
  background-repeat: no-repeat;
  pointer-events: none;
}
.setup-bg::after {
  content: '';
  position: absolute;
  inset: 0;
  background: oklch(0 0 0 / 0.35);
}
.setup-glow {
  position: absolute;
  inset: 0;
  background:
    radial-gradient(700px 420px at 18% 12%, var(--fp-brand-soft), transparent 65%),
    radial-gradient(520px 360px at 85% 88%, color-mix(in srgb, var(--fp-brand-soft) 55%, transparent), transparent 60%);
  pointer-events: none;
}

.setup-container {
  position: relative;
  z-index: 1;
  display: grid;
  grid-template-columns: 1.1fr 1fr;
  width: 960px;
  max-width: 100%;
  min-height: 560px;
  border-radius: var(--fp-radius-lg);
  background: var(--fp-bg-elevated);
  border: 1px solid var(--fp-border);
  box-shadow: var(--fp-shadow-lg);
  overflow: hidden;
}

/* 左侧品牌区（与 LoginView 同构） */
.setup-brand {
  background:
    radial-gradient(420px 300px at 30% 20%, var(--fp-brand-soft), transparent 70%),
    var(--fp-bg-sidebar);
  color: oklch(0.95 0 0);
  display: flex;
  align-items: center;
}
.setup-brand__inner {
  padding: var(--fp-space-8);
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-3);
}
.logo-mark {
  display: inline-flex;
  filter: drop-shadow(0 4px 20px var(--fp-brand-soft));
}
.setup-brand__name {
  font-size: 26px;
  font-weight: 700;
  letter-spacing: 0.02em;
}
.setup-brand__slogan {
  font-size: 13.5px;
  color: oklch(0.8 0.02 250);
  max-width: 280px;
}
.setup-brand__features {
  list-style: none;
  margin-top: var(--fp-space-4);
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-3);
}
.setup-brand__features li {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
  font-size: 13px;
  color: oklch(0.85 0.02 250);
}
.setup-brand__features li i {
  color: var(--fp-brand);
  font-size: 14px;
}

/* 右侧向导 */
.setup-panel {
  display: flex;
  flex-direction: column;
  padding: var(--fp-space-6);
  min-width: 0;
}
.setup-progress {
  height: 3px;
  border-radius: 999px;
  background: var(--fp-border);
  overflow: hidden;
  margin-bottom: var(--fp-space-4);
}
.setup-progress__bar {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: var(--fp-brand-gradient);
  transition: width var(--fp-transition-base);
}
.setup-steps {
  list-style: none;
  display: flex;
  gap: var(--fp-space-2);
  margin-bottom: var(--fp-space-5);
}
.setup-step {
  display: flex;
  align-items: center;
  font-size: 12px;
  color: var(--fp-text-secondary);
  flex: 1;
  min-width: 0;
}
.setup-step__btn {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  min-width: 0;
  padding: 4px 6px;
  margin: -4px -6px;
  border: none;
  border-radius: var(--fp-radius-sm);
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: default;
}
.setup-step.clickable .setup-step__btn {
  cursor: pointer;
}
.setup-step.clickable .setup-step__btn:hover {
  background: var(--fp-bg-hover);
  color: var(--fp-text-primary);
}
.setup-step__btn:focus-visible {
  outline: none;
  box-shadow: var(--fp-focus-ring);
}
.setup-resumed {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
  margin-bottom: var(--fp-space-4);
  padding: var(--fp-space-2) var(--fp-space-3);
  border-radius: var(--fp-radius-sm);
  background: var(--fp-info-soft);
  color: var(--fp-text-secondary);
  font-size: 12.5px;
}
.setup-resumed__reset {
  margin-left: auto;
  border: none;
  background: transparent;
  color: var(--fp-brand);
  font: inherit;
  font-size: 12.5px;
  cursor: pointer;
  text-decoration: underline;
  text-underline-offset: 2px;
}
.setup-nav__hint {
  font-size: 12px;
  color: var(--fp-text-muted);
  align-self: center;
}
.setup-step__dot {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: 50%;
  background: var(--fp-bg-elevated);
  border: 1px solid var(--fp-border);
  font-size: 11px;
  flex-shrink: 0;
}
.setup-step.active {
  color: var(--fp-text-primary);
}
.setup-step.active .setup-step__dot {
  background: var(--fp-brand);
  border-color: var(--fp-brand);
  color: var(--fp-brand-fg);
  box-shadow: 0 0 0 3px var(--fp-brand-ring);
}
.setup-step.done .setup-step__dot {
  background: var(--fp-success);
  border-color: var(--fp-success);
  color: oklch(0.99 0 0);
}
.setup-step__label {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.setup-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  justify-content: center;
}
.setup-nav {
  display: flex;
  justify-content: space-between;
  gap: var(--fp-space-3);
  margin-top: var(--fp-space-5);
}

@media (max-width: 720px) {
  .setup-container {
    grid-template-columns: 1fr;
    max-width: 440px;
  }
  .setup-brand {
    display: none;
  }
  .setup-step__label {
    display: none;
  }
}
</style>
