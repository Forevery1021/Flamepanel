<template>
  <!-- 新建 / 编辑：同一套字段，两处弹窗由 v-if 分流（标题与提交目标不同） -->
  <FpModal v-model="createVisible" :header="t('website.create')" size="md">
    <WebsiteFormFields
      v-model="createForm"
      :errors="createErrors"
      :engine-options="engineOptions"
    />
    <template #footer>
      <FpButton variant="ghost" @click="createVisible = false">{{ t('common.cancel') }}</FpButton>
      <FpButton variant="primary" :loading="submitting" @click="submitCreate">
        {{ t('common.confirm') }}
      </FpButton>
    </template>
  </FpModal>

  <FpModal v-model="editVisible" :header="t('website.edit')" size="md">
    <WebsiteFormFields v-model="editForm" :errors="editErrors" :engine-options="engineOptions" />
    <template #footer>
      <FpButton variant="ghost" @click="editVisible = false">{{ t('common.cancel') }}</FpButton>
      <FpButton variant="primary" :loading="submitting" @click="submitEdit">
        {{ t('common.confirm') }}
      </FpButton>
    </template>
  </FpModal>

  <!-- 切换引擎 -->
  <FpModal v-model="switchVisible" :header="t('website.switchEngine')" size="sm">
    <div class="modal-form">
      <FpSelect
        v-model="switchEngine"
        :label="t('website.engine')"
        :options="engines"
        option-label="name"
        option-value="name"
      />
    </div>
    <template #footer>
      <FpButton variant="ghost" @click="switchVisible = false">{{ t('common.cancel') }}</FpButton>
      <FpButton variant="primary" :loading="submitting" @click="submitSwitchEngine">
        {{ t('common.confirm') }}
      </FpButton>
    </template>
  </FpModal>

  <!-- 站点绑定（运行时 / 项目 / 伪静态模板） -->
  <FpModal v-model="bindVisible" :header="t('website.bind')" size="md">
    <BindForm
      v-model="bindSiteType"
      v-model:runtime-id="bindRuntimeId"
      v-model:project-id="bindProjectId"
      v-model:rewrite-preset="bindRewritePreset"
      :php-runtime-options="phpRuntimeOptions"
      :project-options="projectOptions"
      :rewrite-preset-options="rewritePresetOptions"
    />
    <template #footer>
      <FpButton variant="ghost" @click="bindVisible = false">{{ t('common.cancel') }}</FpButton>
      <FpButton variant="primary" :loading="binding" @click="submitBind">{{
        t('common.confirm')
      }}</FpButton>
    </template>
  </FpModal>

  <!-- 伪静态套用：展示当前规则文本，可套用/清除 -->
  <FpModal v-model="rewriteVisible" :header="t('website.rewritePreset')" size="md">
    <div class="modal-form">
      <FpInlineMessage severity="info" :text="t('website.rewriteHint')" />
      <FpSelect
        v-model="rewritePreset"
        :label="t('website.rewritePreset')"
        :options="rewriteDialogOptions"
        option-label="label"
        option-value="value"
        show-clear
        :placeholder="t('website.rewriteNone')"
      />
      <div v-if="rewriteStatus.rules" class="field-col">
        <label class="field-label">{{ t('website.rewriteCurrent') }}</label>
        <pre class="rewrite-preview">{{ rewriteStatus.rules }}</pre>
      </div>
    </div>
    <template #footer>
      <FpButton variant="ghost" @click="rewriteVisible = false">{{ t('common.cancel') }}</FpButton>
      <FpButton
        v-if="rewriteStatus.preset"
        variant="ghost"
        :loading="rewriting"
        @click="submitClearRewrite"
        >{{ t('website.rewriteClear') }}</FpButton
      >
      <FpButton
        variant="primary"
        :loading="rewriting"
        :disabled="!rewritePreset"
        @click="submitApplyRewrite"
      >
        {{ t('common.confirm') }}
      </FpButton>
    </template>
  </FpModal>
</template>

<script setup lang="ts">
import { reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Website, EngineInfo } from '@/types'
import type { SiteType, SiteRewriteStatus } from '@/api/websites'
import type { WebsiteFormPayload } from '@/composables/useWebsiteList'
import type { WebsiteBindPayload } from '@/composables/useWebsiteBinding'
import FpModal from '@/components/ui/FpModal.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpInlineMessage from '@/components/ui/FpInlineMessage.vue'
import WebsiteFormFields from './WebsiteFormFields.vue'
import BindForm from './BindForm.vue'

/**
 * 网站页签的五个弹窗（Doc/23 §3.3 T2-4 视图拆分第二站）。
 *
 * **编辑态归本组件**（本地 `reactive`），提交时把载荷上抛给数据层：
 * 请求、错误文案与确认分支在 `useWebsiteList` / `useWebsiteBinding` /
 * `useWebsiteRewrite`，组件只负责「什么时候发、发什么」。
 *
 * 三条边界取舍：
 * 1. **弹窗可见性留本组件**：成功才关（由父级返回的布尔决定），失败保持打开让用户
 *    修正输入——这是 UI 决策，不属请求层职责；
 * 2. **表单初值在 `openXxx` 里落**（编辑弹窗需要「以当前行为初值」），故不给
 *    `watch(visible)` 加 `immediate`：本组件由父级 `v-if` 之外的常驻挂载方式渲染，
 *    打开动作必然先于 watch 触发；
 * 3. `BrowseForm` / `BindForm` 拆为子组件，避免单文件同时承载「网站表单」「绑定表单」
 *    两套字段布局（两者字段差异大，合并只会让模板更难读）。
 */
withDefaults(
  defineProps<{
    engines: EngineInfo[]
    phpRuntimeOptions: { label: string; value: number }[]
    projectOptions: { label: string; value: number }[]
    /** 绑定弹窗的伪静态候选（接口 supported 优先，未加载回落本地常量） */
    rewritePresetOptions?: { label: string; value: string }[]
    submitting?: boolean
    binding?: boolean
    rewriting?: boolean
  }>(),
  {
    rewritePresetOptions: () => [],
    submitting: false,
    binding: false,
    rewriting: false,
  },
)

const emit = defineEmits<{
  create: [payload: WebsiteFormPayload]
  update: [id: number, payload: WebsiteFormPayload]
  switchEngine: [id: number, engine: string]
  bind: [id: number, payload: WebsiteBindPayload]
  applyRewrite: [id: number, preset: string]
  clearRewrite: [id: number]
}>()

const { t } = useI18n()

/* ── 引擎下拉的静态候选（切换弹窗用 `engines` prop，这里供编辑表单消费） ── */
const engineOptions = [
  { label: 'nginx', value: 'nginx' },
  { label: 'apache', value: 'apache' },
  { label: 'openlitespeed', value: 'openlitespeed' },
  { label: 'openresty', value: 'openresty' },
  { label: 'caddy', value: 'caddy' },
]

/* ── 新建 ── */

const createVisible = ref(false)
const createForm = reactive<WebsiteFormPayload>(emptyForm())
const createErrors = reactive({ name: '', domain: '' })

function emptyForm(): WebsiteFormPayload {
  return {
    name: '',
    domain: '',
    root_path: '',
    node_id: 1,
    engine: 'nginx',
    ssl_enabled: false,
    proxy_enabled: false,
    proxy_pass: '',
    site_type: 'static',
  }
}

/** 打开新建弹窗（每次都是新建会话，重置全部字段与错误） */
function openCreate() {
  Object.assign(createForm, emptyForm())
  createErrors.name = ''
  createErrors.domain = ''
  createVisible.value = true
}

async function submitCreate() {
  if (!validate(createForm, createErrors)) return
  emit('create', { ...createForm })
}

/* ── 编辑 ── */

const editVisible = ref(false)
const editingId = ref(0)
const editForm = reactive<WebsiteFormPayload>(emptyForm())
const editErrors = reactive({ name: '', domain: '' })

/** 打开编辑弹窗：以当前行为初值（字段缺失走与原实现一致的回退值） */
function openEdit(row: Website) {
  editingId.value = row.id
  Object.assign(editForm, {
    name: row.name,
    domain: row.domain,
    root_path: row.root_path,
    node_id: row.node_id,
    engine: row.engine || 'nginx',
    ssl_enabled: row.ssl_enabled,
    proxy_enabled: row.proxy_enabled,
    proxy_pass: row.proxy_pass || '',
    site_type: (row.site_type || 'static') as SiteType,
  } satisfies WebsiteFormPayload)
  editErrors.name = ''
  editErrors.domain = ''
  editVisible.value = true
}

async function submitEdit() {
  if (!validate(editForm, editErrors)) return
  emit('update', editingId.value, { ...editForm })
}

/** 必填校验（两处表单共用；错误写入传入的 errors 对象） */
function validate(form: WebsiteFormPayload, errors: { name: string; domain: string }): boolean {
  errors.name = form.name ? '' : t('common.required')
  errors.domain = form.domain ? '' : t('common.required')
  return !errors.name && !errors.domain
}

/* ── 切换引擎 ── */

const switchVisible = ref(false)
const switchEngine = ref('')
let switchTargetId = 0

function openSwitchEngine(row: Website) {
  switchTargetId = row.id
  switchEngine.value = row.engine
  switchVisible.value = true
}

function submitSwitchEngine() {
  if (!switchEngine.value) return
  emit('switchEngine', switchTargetId, switchEngine.value)
}

/* ── 站点绑定 ── */

const bindVisible = ref(false)
let bindTargetId = 0
const bindSiteType = ref<SiteType>('static')
/** 清空经 FpSelect 上报 `undefined`；提交时归一为 `number | null`（后端协议） */
const bindRuntimeId = ref<string | number | string[] | undefined>(undefined)
const bindProjectId = ref<string | number | string[] | undefined>(undefined)
const bindRewritePreset = ref('')

function openBind(row: Website) {
  bindTargetId = row.id
  bindSiteType.value = (row.site_type || 'static') as SiteType
  bindRuntimeId.value = row.runtime_id ?? undefined
  bindProjectId.value = row.project_id ?? undefined
  bindRewritePreset.value = ''
  bindVisible.value = true
}

/** 绑定目标归一：非数字（清空 / 站点类型不适用）一律 null */
function toId(v: string | number | string[] | undefined): number | null {
  return typeof v === 'number' ? v : null
}

function submitBind() {
  emit('bind', bindTargetId, {
    site_type: bindSiteType.value,
    runtime_id: toId(bindRuntimeId.value),
    project_id: toId(bindProjectId.value),
    rewrite_preset: bindRewritePreset.value || undefined,
  })
}

/* ── 伪静态 ── */

const rewriteVisible = ref(false)
let rewriteTargetId = 0
/** 当前选中的模板（编辑态本地持有） */
const rewritePreset = ref<string | undefined>(undefined)

/**
 * 伪静态服务端状态（规则预览 + 清除按钮显隐）——**打开时快照**，写操作成功后由
 * 父级调 `syncRewriteStatus` 回写。刻意不走 prop 下发：父级是「先 await 拉状态、
 * 再开弹窗」，而父子间 prop 更新要等一次渲染调度，快照语义更确定（也少一次隐式依赖）。
 */
const rewriteStatus = ref<SiteRewriteStatus>({ preset: null, rules: '', supported: [] })
const rewriteDialogOptions = ref<{ label: string; value: string }[]>([])

/**
 * 打开伪静态弹窗：模板初值以**服务端状态**为权威（父级已先 `prepare` 拉取），
 * 服务端未返回时回落到行数据里的 `rewrite_preset`（列表页已带该字段）。
 */
function openRewrite(
  row: Website,
  status: SiteRewriteStatus | null,
  options: { label: string; value: string }[],
) {
  rewriteTargetId = row.id
  rewriteStatus.value = status ?? { preset: null, rules: '', supported: [] }
  rewriteDialogOptions.value = options
  rewritePreset.value = status?.preset ?? row.rewrite_preset ?? undefined
  rewriteVisible.value = true
}

/** 写操作成功后由父级调用：以服务端最新状态回写预览（rules / preset） */
function syncRewriteStatus(status: SiteRewriteStatus) {
  rewriteStatus.value = status
  rewritePreset.value = status.preset ?? undefined
}

function submitApplyRewrite() {
  if (!rewritePreset.value) return
  emit('applyRewrite', rewriteTargetId, rewritePreset.value)
}

function submitClearRewrite() {
  emit('clearRewrite', rewriteTargetId)
}

/** 供父级调用：成功才关（失败保持打开让用户修正输入） */
const closeCreate = () => (createVisible.value = false)
const closeEdit = () => (editVisible.value = false)
const closeSwitchEngine = () => (switchVisible.value = false)
const closeBind = () => (bindVisible.value = false)
const closeRewrite = () => (rewriteVisible.value = false)

defineExpose({
  openCreate,
  openEdit,
  openSwitchEngine,
  openBind,
  openRewrite,
  syncRewriteStatus,
  closeCreate,
  closeEdit,
  closeSwitchEngine,
  closeBind,
  closeRewrite,
})
</script>

<style scoped>
.rewrite-preview {
  margin: 0;
  padding: var(--fp-space-2, 8px);
  background: var(--fp-color-surface-alt, rgba(127, 127, 127, 0.08));
  border-radius: var(--fp-radius-sm, 4px);
  font-size: var(--fp-font-size-xs, 12px);
  line-height: 1.5;
  overflow-x: auto;
  white-space: pre;
}
</style>
