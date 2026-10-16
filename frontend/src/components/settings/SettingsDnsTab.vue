<template>
  <div class="settings-card">
    <div class="dns-toolbar">
      <FpButton
        v-permission="{ perm: 'dns_provider:create', mode: 'view' }"
        variant="primary"
        icon="oi oi-plus"
        @click="openDialog()"
      >
        {{ t('dnsProvider.create') }}
      </FpButton>
    </div>

    <FpTable
      :rows="accounts"
      :loading="loading"
      :paginator="false"
      :empty-text="t('common.noData')"
    >
      <FpColumn field="name" :header="t('dnsProvider.name')" style="min-width: 160px" />
      <FpColumn :header="t('dnsProvider.type')" style="width: 130px">
        <template #body="{ data }">
          <FpTag severity="info" :value="providerLabel(data.provider_type)" />
        </template>
      </FpColumn>
      <FpColumn :header="t('dnsProvider.enabled')" style="width: 90px">
        <template #body="{ data }">
          <FpSwitch
            :model-value="data.enabled"
            @update:model-value="(v: boolean) => onToggle(data, v)"
          />
        </template>
      </FpColumn>
      <FpColumn :header="t('dnsProvider.testStatus')" style="width: 120px">
        <template #body="{ data }">
          <FpTag
            :severity="testTag(data.last_test_status)"
            :value="testLabel(data.last_test_status)"
          />
        </template>
      </FpColumn>
      <FpColumn :header="t('dnsProvider.testedAt')" style="width: 170px">
        <template #body="{ data }">
          <span>{{ data.last_test_at ? new Date(data.last_test_at).toLocaleString() : '-' }}</span>
        </template>
      </FpColumn>
      <FpColumn :header="t('common.colActions')" style="width: 260px" frozen>
        <template #body="{ data }">
          <div class="row-actions">
            <FpButton
              v-permission="{ perm: 'dns_provider:update', mode: 'view' }"
              variant="primary"
              :loading="testingId === data.id"
              @click="onTest(data)"
            >
              {{ t('dnsProvider.test') }}
            </FpButton>
            <FpButton
              v-permission="{ perm: 'dns_provider:update', mode: 'view' }"
              variant="link"
              @click="openDialog(data)"
            >
              {{ t('common.edit') }}
            </FpButton>
            <FpButton
              v-permission="{ perm: 'dns_provider:delete', mode: 'view' }"
              variant="link"
              @click="onDelete(data)"
            >
              {{ t('common.delete') }}
            </FpButton>
          </div>
        </template>
      </FpColumn>
    </FpTable>

    <!-- 创建/编辑弹窗（编辑时凭据留空 = 不修改，后端不回显敏感凭据） -->
    <FpModal
      v-model="dialogVisible"
      :header="editingId ? t('dnsProvider.edit') : t('dnsProvider.create')"
      size="md"
    >
      <div class="modal-form">
        <FpInput v-model="form.name" :label="t('dnsProvider.name')" :error="errors.name" />
        <FpSelect
          v-model="form.provider_type"
          :label="t('dnsProvider.type')"
          :options="providerTypeOptions"
          option-label="label"
          option-value="value"
        />
        <div class="field-col">
          <FpTextarea
            v-model="form.credentials_json"
            :label="t('dnsProvider.credentials')"
            :rows="5"
            :invalid="!!errors.credentials_json"
            :placeholder="credentialsPlaceholder"
          />
          <small v-if="errors.credentials_json" class="field-error">{{
            errors.credentials_json
          }}</small>
          <small v-else-if="editingId" class="field-hint">{{
            t('dnsProvider.credentialsEditHint')
          }}</small>
        </div>
        <FpSwitch v-model="form.enabled" :label="t('dnsProvider.enabled')" />
      </div>
      <template #footer>
        <FpButton variant="ghost" @click="dialogVisible = false">{{ t('common.cancel') }}</FpButton>
        <FpButton variant="primary" :loading="saving" @click="save">{{
          t('common.save')
        }}</FpButton>
      </template>
    </FpModal>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  listDnsProviders,
  createDnsProvider,
  updateDnsProvider,
  deleteDnsProvider,
  testDnsProvider,
} from '@/api/dnsProviders'
import type { DnsProviderAccountDto } from '@/api/generated'
import { useApiQuery, useQueryCacheClient } from '@/composables/useApiQuery'
import { queryKeys } from '@/api/queryKeys'
import FpTable from '@/components/ui/FpTable.vue'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpTextarea from '@/components/ui/FpTextarea.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpTag from '@/components/ui/FpTag.vue'
import FpSwitch from '@/components/ui/FpSwitch.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'

type TagSeverity = 'success' | 'warning' | 'danger' | 'info' | 'neutral'

const { t } = useI18n()
const toast = useFpToast()
const { confirmAction } = useFpConfirm()
const queryClient = useQueryCacheClient()

const accountsQuery = useApiQuery<DnsProviderAccountDto[]>(
  () => queryKeys.dnsProviders.list(),
  async () => {
    const res = await listDnsProviders()
    return { data: res.data }
  },
)
const accounts = computed<DnsProviderAccountDto[]>(() => accountsQuery.data.value ?? [])
const loading = accountsQuery.loading

const providerTypeOptions = computed(() => [
  { label: 'Cloudflare', value: 'cloudflare' },
  { label: '阿里云', value: 'aliyun' },
  { label: '腾讯云', value: 'tencent' },
  { label: '华为云', value: 'huawei' },
])

function providerLabel(type: string): string {
  return providerTypeOptions.value.find((p) => p.value === type)?.label ?? type
}

const credentialsPlaceholder = computed(() => {
  const map: Record<string, string> = {
    cloudflare: '{"api_token":"..."}',
    aliyun: '{"access_key_id":"...","access_key_secret":"..."}',
    tencent: '{"secret_id":"...","secret_key":"..."}',
    huawei: '{"access_key":"...","secret_key":"..."}',
  }
  return map[form.provider_type] ?? '{...}'
})

function testTag(status: string | null): TagSeverity {
  if (status === 'success') return 'success'
  if (status === 'failed') return 'danger'
  return 'neutral'
}

function testLabel(status: string | null) {
  if (status === 'success') return t('backup.testSuccess')
  if (status === 'failed') return t('backup.testFailed')
  return t('backup.testNever')
}

/* ── 创建/编辑弹窗 ── */
const dialogVisible = ref(false)
const saving = ref(false)
const editingId = ref(0)
const testingId = ref<number | null>(null)
const form = reactive({
  name: '',
  provider_type: 'cloudflare',
  credentials_json: '',
  enabled: true,
})
const errors = reactive({ name: '', credentials_json: '' })

function openDialog(row?: DnsProviderAccountDto) {
  editingId.value = row?.id ?? 0
  form.name = row?.name ?? ''
  form.provider_type = row?.provider_type ?? 'cloudflare'
  // 编辑时留空 = 不修改凭据（后端不回显敏感 credentials_json）
  form.credentials_json = ''
  form.enabled = row?.enabled ?? true
  errors.name = ''
  errors.credentials_json = ''
  dialogVisible.value = true
}

function validate(): boolean {
  errors.name = form.name.trim() ? '' : t('common.required')
  // 新建时必须提供凭据；编辑时留空表示不修改
  errors.credentials_json =
    !editingId.value && !form.credentials_json.trim() ? t('common.required') : ''
  return !errors.name && !errors.credentials_json
}

async function save() {
  if (!validate()) return
  saving.value = true
  try {
    if (editingId.value) {
      await updateDnsProvider(editingId.value, {
        name: form.name.trim(),
        provider_type: form.provider_type,
        credentials_json: form.credentials_json.trim() || undefined,
        enabled: form.enabled,
      })
    } else {
      await createDnsProvider({
        name: form.name.trim(),
        provider_type: form.provider_type,
        credentials_json: form.credentials_json.trim(),
        enabled: form.enabled,
      })
    }
    toast.success(t('common.success'))
    dialogVisible.value = false
    queryClient.invalidateQueries({ queryKey: queryKeys.dnsProviders.all })
  } catch (err) {
    toast.error(err, t('common.failed'))
  } finally {
    saving.value = false
  }
}

function onToggle(row: DnsProviderAccountDto, enabled: boolean) {
  confirmAction({
    message: enabled
      ? t('dnsProvider.enableConfirm', { name: row.name })
      : t('dnsProvider.disableConfirm', { name: row.name }),
    header: t('common.confirm'),
    accept: async () => {
      try {
        await updateDnsProvider(row.id, { enabled })
        toast.success(t('common.success'))
        queryClient.invalidateQueries({ queryKey: queryKeys.dnsProviders.all })
      } catch {
        toast.error(t('common.failed'))
        queryClient.invalidateQueries({ queryKey: queryKeys.dnsProviders.all })
      }
    },
  })
}

async function onTest(row: DnsProviderAccountDto) {
  testingId.value = row.id
  try {
    const res = await testDnsProvider(row.id)
    if (res.data.last_test_status === 'success') {
      toast.success(t('backup.testSuccess'))
    } else {
      toast.error(t('backup.testFailed'))
    }
    queryClient.invalidateQueries({ queryKey: queryKeys.dnsProviders.all })
  } catch (err) {
    toast.error(err, t('common.failed'))
  } finally {
    testingId.value = null
  }
}

function onDelete(row: DnsProviderAccountDto) {
  confirmAction({
    message: t('dnsProvider.deleteConfirm', { name: row.name }),
    header: t('common.confirm'),
    acceptLabel: t('common.delete'),
    rejectLabel: t('common.cancel'),
    accept: async () => {
      try {
        await deleteDnsProvider(row.id)
        toast.success(t('common.success'))
        queryClient.invalidateQueries({ queryKey: queryKeys.dnsProviders.all })
      } catch (err) {
        toast.error(err, t('common.failed'))
      }
    },
  })
}
</script>

<style scoped>
.dns-toolbar {
  display: flex;
  justify-content: flex-end;
  margin-bottom: 0.75rem;
}
</style>
