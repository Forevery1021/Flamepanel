<template>
  <!-- ACME 签发 -->
  <FpModal v-model="acmeVisible" :header="t('certificate.issueAcme')" size="md">
    <div class="modal-form">
      <FpInput v-model="acmeForm.domains" :label="t('certificate.primaryDomain')" />
      <FpInput v-model="acmeAlt" :label="t('certificate.altDomains')" />
      <FpSwitch v-model="acmeForm.dns01" :label="t('certificate.dns01')" />
      <FpInlineMessage v-if="acmeForm.dns01" severity="info" :text="t('certificate.dns01Hint')" />
      <FpSelect
        v-if="acmeForm.dns01"
        v-model="acmeForm.dns_provider_account_id"
        :label="t('certificate.dnsAccount')"
        :options="dnsAccountOptions"
        option-label="label"
        option-value="value"
        show-clear
        :placeholder="t('certificate.dnsAccountPlaceholder')"
      />
      <FpInput
        v-if="!acmeForm.dns01"
        v-model="acmeForm.webroot"
        :label="t('certificate.webroot')"
      />
    </div>
    <template #footer>
      <FpButton variant="ghost" @click="acmeVisible = false">{{ t('common.cancel') }}</FpButton>
      <FpButton variant="primary" :disabled="submitting" @click="submitAcme">{{
        t('certificate.issue')
      }}</FpButton>
    </template>
  </FpModal>

  <!-- 自签 -->
  <FpModal v-model="selfVisible" :header="t('certificate.issueSelfSigned')" size="md">
    <div class="modal-form">
      <FpInput v-model="selfForm.domains" :label="t('certificate.primaryDomain')" />
      <FpInput v-model="selfForm.alt" :label="t('certificate.altDomains')" />
    </div>
    <template #footer>
      <FpButton variant="ghost" @click="selfVisible = false">{{ t('common.cancel') }}</FpButton>
      <FpButton variant="primary" :disabled="submitting" @click="submitSelf">{{
        t('certificate.issue')
      }}</FpButton>
    </template>
  </FpModal>

  <!-- 上传第三方证书 -->
  <FpModal v-model="uploadVisible" :header="t('certificate.upload')" size="md">
    <div class="modal-form">
      <FpInput v-model="uploadForm.domains" :label="t('certificate.domains')" />
      <FpTextarea v-model="uploadForm.cert_pem" :label="t('certificate.certPem')" :rows="6" />
      <FpTextarea v-model="uploadForm.key_pem" :label="t('certificate.keyPem')" :rows="6" />
    </div>
    <template #footer>
      <FpButton variant="ghost" @click="uploadVisible = false">{{ t('common.cancel') }}</FpButton>
      <FpButton variant="primary" :disabled="submitting" @click="submitUpload">{{
        t('certificate.upload')
      }}</FpButton>
    </template>
  </FpModal>

  <!-- 绑定网站 -->
  <FpModal v-model="bindVisible" :header="t('certificate.bind')" size="sm">
    <div class="modal-form">
      <div class="field-col">
        <label class="field-label">{{ t('certificate.boundWebsite') }}</label>
        <FpNumber v-model="bindWebsiteId" :min="1" class="w-full" />
      </div>
    </div>
    <template #footer>
      <FpButton variant="ghost" @click="bindVisible = false">{{ t('common.cancel') }}</FpButton>
      <FpButton variant="primary" :disabled="submitting" @click="submitBind">{{
        t('common.confirm')
      }}</FpButton>
    </template>
  </FpModal>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Certificate } from '@/api/certificates'
import type { DnsProviderAccountDto } from '@/api/generated'
import FpModal from '@/components/ui/FpModal.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpSwitch from '@/components/ui/FpSwitch.vue'
import FpNumber from '@/components/ui/FpNumber.vue'
import FpTextarea from '@/components/ui/FpTextarea.vue'
import FpInlineMessage from '@/components/ui/FpInlineMessage.vue'

/**
 * 证书四个弹窗（Doc/23 §3.3 T2-4 视图拆分第二站；主线三 §6.1 Stage 2/3）。
 *
 * **编辑态归本组件**，提交时上抛载荷：请求与 toast 在 `useWebsiteCertificates`。
 *
 * 一处易被忽略的语义（原实现即如此）：**弹窗可见性由父级 `v-model` 之外的本组件
 * 持有**，但「成功才关」的判定在父级——父级拿到的返回值决定是否调 `closeXxx()`。
 * 这样失败时输入保持原样，用户可修正后重试。
 *
 * 域名输入的两个细节与原实现一致：主域名必填（空则 `required` 提示，不关弹窗）；
 * 备用域名按逗号拆分并**过滤空项**（`"a.com, ,b.com"` 不会产生空串 SAN）。
 */
const props = defineProps<{
  submitting?: boolean
  dnsAccounts?: DnsProviderAccountDto[]
}>()

const emit = defineEmits<{
  acme: [
    payload: {
      domains: string[]
      dns01: boolean
      webroot?: string
      dns_provider_account_id?: number
    },
  ]
  selfSigned: [domains: string[]]
  upload: [payload: { domains: string[]; cert_pem: string; key_pem: string }]
  bind: [id: number, websiteId: number | null]
  required: []
}>()

const { t } = useI18n()

/** 下拉候选：仅启用项（凭据永远不回显，故只需 name / provider_type / id） */
const dnsAccountOptions = computed(() =>
  (props.dnsAccounts ?? [])
    .filter((a) => a.enabled)
    .map((a) => ({ label: `${a.name} (${a.provider_type})`, value: a.id })),
)

/* ── ACME ── */

const acmeVisible = ref(false)
const acmeAlt = ref('')
const acmeForm = reactive({
  domains: '',
  dns01: false,
  webroot: '',
  dns_provider_account_id: undefined as number | undefined,
})

function openAcme() {
  acmeForm.domains = ''
  acmeForm.dns01 = false
  acmeForm.webroot = ''
  acmeForm.dns_provider_account_id = undefined
  acmeAlt.value = ''
  acmeVisible.value = true
}

function submitAcme() {
  const primary = acmeForm.domains.trim()
  if (!primary) {
    emit('required')
    return
  }
  emit('acme', {
    domains: [primary, ...splitAlt(acmeAlt.value)],
    dns01: acmeForm.dns01,
    webroot: acmeForm.webroot,
    dns_provider_account_id: acmeForm.dns_provider_account_id,
  })
}

/* ── 自签 ── */

const selfVisible = ref(false)
const selfForm = reactive({ domains: '', alt: '' })

function openSelf() {
  selfForm.domains = ''
  selfForm.alt = ''
  selfVisible.value = true
}

function submitSelf() {
  const primary = selfForm.domains.trim()
  if (!primary) {
    emit('required')
    return
  }
  emit('selfSigned', [primary, ...splitAlt(selfForm.alt)])
}

/* ── 上传 ── */

const uploadVisible = ref(false)
const uploadForm = reactive({ domains: '', cert_pem: '', key_pem: '' })

function openUpload() {
  uploadForm.domains = ''
  uploadForm.cert_pem = ''
  uploadForm.key_pem = ''
  uploadVisible.value = true
}

function submitUpload() {
  const primary = uploadForm.domains.trim()
  // 三个字段全必填（私钥缺失时后端必然拒绝，前端先拦一道避免无效请求）
  if (!primary || !uploadForm.cert_pem || !uploadForm.key_pem) {
    emit('required')
    return
  }
  const domains = splitAlt(uploadForm.domains)
  emit('upload', {
    domains: domains.length ? domains : [primary],
    cert_pem: uploadForm.cert_pem,
    key_pem: uploadForm.key_pem,
  })
}

/* ── 绑定网站 ── */

const bindVisible = ref(false)
const bindWebsiteId = ref<number | undefined>(undefined)
let bindTargetId = 0

function openBind(row: Certificate) {
  bindTargetId = row.id
  bindWebsiteId.value = row.website_id ?? undefined
  bindVisible.value = true
}

function submitBind() {
  emit('bind', bindTargetId, typeof bindWebsiteId.value === 'number' ? bindWebsiteId.value : null)
}

/* ── 便捷出口 ── */

/** 逗号分隔域名 → 去空项数组（主域名单独处理，不在此列） */
function splitAlt(v: string): string[] {
  return v
    .split(',')
    .map((d) => d.trim())
    .filter(Boolean)
}

function closeAll() {
  acmeVisible.value = false
  selfVisible.value = false
  uploadVisible.value = false
  bindVisible.value = false
}

defineExpose({
  openAcme,
  openSelf,
  openUpload,
  openBind,
  closeAcme: () => (acmeVisible.value = false),
  closeSelf: () => (selfVisible.value = false),
  closeUpload: () => (uploadVisible.value = false),
  closeBind: () => (bindVisible.value = false),
  closeAll,
})
</script>
