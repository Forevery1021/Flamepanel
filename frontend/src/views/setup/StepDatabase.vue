<template>
  <div class="step">
    <h2 class="step__title">{{ t('setup.dbTitle') }}</h2>
    <p class="step__sub">{{ t('setup.dbSub') }}</p>

    <div class="step-db-type">
      <label
        v-for="opt in dbOptions"
        :key="opt.value"
        class="db-option"
        :class="{ selected: db.db_type === opt.value }"
      >
        <input
          type="radio"
          :value="opt.value"
          :checked="db.db_type === opt.value"
          @change="emit('update', { db_type: opt.value })"
        />
        <span class="db-option__name">{{ t(opt.labelKey) }}</span>
        <span class="db-option__desc">{{ t(opt.descKey) }}</span>
      </label>
    </div>

    <template v-if="db.db_type !== 'sqlite'">
      <div class="step-form">
        <div class="step-form__row">
          <FpInput
            v-model="host"
            :label="t('setup.dbHost')"
            @update:model-value="emit('update', { host: $event })"
          />
          <FpInput
            v-model="port"
            :label="t('setup.dbPort')"
            type="number"
            @update:model-value="emit('update', { port: Number($event) || 3306 })"
          />
        </div>
        <FpInput
          v-model="name"
          :label="t('setup.dbName')"
          @update:model-value="emit('update', { name: $event })"
        />
        <FpInput
          v-model="user"
          :label="t('setup.dbUser')"
          @update:model-value="emit('update', { user: $event })"
        />
        <FpInput
          v-model="password"
          :label="t('setup.dbPassword')"
          type="password"
          toggle-mask
          @update:model-value="emit('update', { password: $event })"
        />
        <FpInput
          v-model="mysqlRootPassword"
          :label="t('setup.dbRootPassword')"
          type="password"
          toggle-mask
          @update:model-value="emit('update', { mysql_root_password: $event })"
        />
      </div>
      <p class="step-hint">{{ t('setup.dbRootHint') }}</p>
    </template>
    <p v-else class="step-hint">{{ t('setup.dbSqliteHint') }}</p>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import FpInput from '@/components/ui/FpInput.vue'

export interface SetupDatabaseForm {
  db_type: 'sqlite' | 'mysql' | 'mariadb'
  host: string
  port: number
  name: string
  user: string
  password: string
  mysql_root_password: string
}

const props = defineProps<{ db: SetupDatabaseForm }>()
const emit = defineEmits<{ update: [patch: Partial<SetupDatabaseForm>] }>()

const { t } = useI18n()
const dbOptions = [
  { value: 'sqlite' as const, labelKey: 'setup.dbSqlite', descKey: 'setup.dbSqliteDesc' },
  { value: 'mysql' as const, labelKey: 'setup.dbMysql', descKey: 'setup.dbMysqlDesc' },
  { value: 'mariadb' as const, labelKey: 'setup.dbMariaDB', descKey: 'setup.dbMariaDBDesc' },
]

const host = computed({ get: () => props.db.host, set: (v) => emit('update', { host: v }) })
const port = computed({
  get: () => String(props.db.port),
  set: (v) => emit('update', { port: Number(v) || 3306 }),
})
const name = computed({ get: () => props.db.name, set: (v) => emit('update', { name: v }) })
const user = computed({ get: () => props.db.user, set: (v) => emit('update', { user: v }) })
const password = computed({
  get: () => props.db.password,
  set: (v) => emit('update', { password: v }),
})
const mysqlRootPassword = computed({
  get: () => props.db.mysql_root_password,
  set: (v) => emit('update', { mysql_root_password: v }),
})
</script>

<style scoped>
.step {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-4);
}
.step__title {
  font-size: 20px;
  font-weight: 700;
  color: var(--fp-text-primary);
}
.step__sub {
  font-size: 13px;
  color: var(--fp-text-secondary);
}
.step-db-type {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: var(--fp-space-3);
}
.db-option {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: var(--fp-space-4);
  border-radius: var(--fp-radius-md);
  background: var(--fp-bg-elevated);
  border: 1px solid var(--fp-border);
  cursor: pointer;
  transition: border-color 0.15s;
}
.db-option:hover {
  border-color: var(--fp-brand);
}
.db-option.selected {
  border-color: var(--fp-brand);
  background: var(--fp-brand-soft);
}
.db-option input {
  display: none;
}
.db-option__name {
  font-size: 13px;
  font-weight: 600;
  color: var(--fp-text-primary);
}
.db-option__desc {
  font-size: 12px;
  color: var(--fp-text-secondary);
}
.step-form {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-4);
  max-width: 420px;
  margin-top: var(--fp-space-2);
}
.step-form__row {
  display: grid;
  grid-template-columns: 2fr 1fr;
  gap: var(--fp-space-3);
}
.step-hint {
  font-size: 12.5px;
  color: var(--fp-text-secondary);
}

@media (max-width: 480px) {
  .step-db-type {
    grid-template-columns: 1fr;
  }
  .step-form__row {
    grid-template-columns: 1fr;
  }
}
</style>
