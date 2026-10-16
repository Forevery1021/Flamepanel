<template>
  <FpModal v-model="visible" :header="engine ? t(meta.titleKey) : ''">
    <div class="modal-form">
      <FpInput
        v-model="form.name"
        :label="t('database.name')"
        :placeholder="t('common.placeholder')"
      />
      <FpSelect
        v-if="meta.versions"
        v-model="form.version"
        :label="t('database.version')"
        :options="meta.versions"
      />
      <FpInput
        v-else
        v-model="form.version"
        :label="t('database.version')"
        :placeholder="t('database.versionPlaceholder')"
      />
      <div class="field-col">
        <label class="field-label">{{ t('database.port') }}</label>
        <FpNumber v-model="form.port" :min="1024" :max="65535" class="w-full" />
      </div>
      <FpInput
        v-model="form.password"
        :label="engine === 'mysql' ? t('database.rootPassword') : t('database.password')"
        type="password"
        toggle-mask
        :placeholder="meta.passwordOptional ? t('database.optional') : ''"
        :error="
          passwordTooShort ? t('database.passwordTooShort', { n: meta.minPasswordLength }) : ''
        "
      />
      <small v-if="meta.passwordHintKey" class="field-hint">{{ t(meta.passwordHintKey) }}</small>
    </div>
    <template #footer>
      <FpButton variant="ghost" @click="visible = false">{{ t('common.cancel') }}</FpButton>
      <FpButton variant="primary" :loading="installing" :disabled="!canSubmit" @click="submit">
        {{ t('common.install') }}
      </FpButton>
    </template>
  </FpModal>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpNumber from '@/components/ui/FpNumber.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpButton from '@/components/ui/FpButton.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { installMongodb, installMysql, installPostgres, installRedis } from '@/api/databases'
import {
  DATABASE_ENGINES,
  defaultInstallForm,
  installPayload,
  passwordValid,
  type DatabaseEngine,
} from '@/utils/databases'

/**
 * 统一的数据库安装弹窗（MySQL / Redis / PostgreSQL / MongoDB）。
 * `engine` 为 null 时关闭；打开时按引擎重置默认名称与端口。成功后 emit `done`。
 */
const props = defineProps<{ engine: DatabaseEngine | null }>()
const emit = defineEmits<{ 'update:engine': [DatabaseEngine | null]; done: [] }>()

const { t } = useI18n()
const toast = useFpToast()

const visible = computed({
  get: () => props.engine !== null,
  set: (v: boolean) => {
    if (!v) emit('update:engine', null)
  },
})
const meta = computed(() => DATABASE_ENGINES[props.engine ?? 'mysql'])
const form = ref(defaultInstallForm('mysql'))
const installing = ref(false)
const passwordTooShort = computed(
  () =>
    props.engine !== null &&
    form.value.password !== '' &&
    !passwordValid(props.engine, form.value.password),
)
const canSubmit = computed(
  () => form.value.name.trim() !== '' && form.value.port > 0 && !passwordTooShort.value,
)

watch(
  () => props.engine,
  (engine) => {
    if (engine) form.value = defaultInstallForm(engine)
  },
  { immediate: true },
)

const installers = {
  mysql: installMysql,
  redis: installRedis,
  postgres: installPostgres,
  mongodb: installMongodb,
} as const

async function submit() {
  const engine = props.engine
  if (!engine || !canSubmit.value) return
  installing.value = true
  try {
    await installers[engine](installPayload(engine, form.value))
    toast.success(t('common.success'))
    emit('update:engine', null)
    emit('done')
  } catch (err) {
    toast.error(err, t('common.failed'))
  } finally {
    installing.value = false
  }
}
</script>

<style scoped>
.field-hint {
  display: block;
  margin-top: -0.25rem;
  color: var(--fp-text-muted);
  font-size: 0.8125rem;
}
</style>
