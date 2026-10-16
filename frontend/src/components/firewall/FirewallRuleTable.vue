<template>
  <FpTable
    :rows="rules"
    :loading="loading"
    :first="first"
    :rows-per-page="pageSize"
    :empty-text="t('common.noData')"
    :empty-desc="t('pageDesc.emptyHint')"
    @update:first="emit('update:first', $event)"
  >
    <template #emptyAction>
      <FpButton
        v-permission="{ perm: 'firewall:create', mode: 'view' }"
        variant="primary"
        icon="oi oi-plus"
        @click="emit('create')"
        >{{ t('firewall.add') }}</FpButton
      >
    </template>
    <FpColumn header="#" style="width: 50px">
      <template #body="{ index }">{{ index + 1 }}</template>
    </FpColumn>
    <FpColumn field="name" :header="t('firewall.name')" />
    <FpColumn :header="t('firewall.protocol')" style="width: 90px">
      <template #body="{ data }">
        <FpTag
          :severity="data.protocol === 'any' ? 'neutral' : 'info'"
          :value="data.protocol.toUpperCase()"
        />
      </template>
    </FpColumn>
    <FpColumn :header="t('firewall.port')" style="width: 120px">
      <template #body="{ data }">
        <span :class="{ mono: data.port }">{{ data.port || t('firewall.any') }}</span>
      </template>
    </FpColumn>
    <FpColumn :header="t('firewall.source')" style="width: 150px">
      <template #body="{ data }">
        <span class="mono">{{ data.source || '0.0.0.0/0' }}</span>
      </template>
    </FpColumn>
    <FpColumn :header="t('firewall.action')" style="width: 100px">
      <template #body="{ data }">
        <FpTag
          :severity="data.action === 'allow' ? 'success' : 'danger'"
          :value="actionLabel(data.action)"
        />
      </template>
    </FpColumn>
    <FpColumn :header="t('firewall.direction')" style="width: 90px">
      <template #body="{ data }">
        {{ data.direction === 'in' ? t('firewall.in') : t('firewall.out') }}
      </template>
    </FpColumn>
    <FpColumn :header="t('firewall.enabled')" style="width: 90px">
      <template #body="{ data }">
        <FpSwitch
          :model-value="data.enabled"
          @update:model-value="(v: boolean) => emit('toggle', data, v)"
        />
      </template>
    </FpColumn>
    <FpColumn :header="t('firewall.actions')" style="width: 150px" frozen>
      <template #body="{ data }">
        <div class="row-actions">
          <FpButton
            v-permission="{ perm: 'firewall:update', mode: 'view' }"
            variant="link"
            @click="emit('edit', data)"
            >{{ t('common.edit') }}</FpButton
          >
          <FpButton
            v-permission="{ perm: 'firewall:delete', mode: 'view' }"
            variant="link"
            @click="emit('delete', data)"
          >
            {{ t('firewall.delete') }}
          </FpButton>
        </div>
      </template>
    </FpColumn>
  </FpTable>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'

import FpTable from '@/components/ui/FpTable.vue'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpTag from '@/components/ui/FpTag.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpSwitch from '@/components/ui/FpSwitch.vue'
import type { FirewallRule } from '@/types'

/** 防火墙规则表格（纯展示）：协议 / 端口 / 地址 / 动作 / 方向 / 启用开关 / 编辑删除。 */
defineProps<{
  rules: FirewallRule[]
  loading: boolean
  first: number
  pageSize: number
}>()

const emit = defineEmits<{
  create: []
  edit: [rule: FirewallRule]
  delete: [rule: FirewallRule]
  toggle: [rule: FirewallRule, enabled: boolean]
  'update:first': [value: number]
}>()

const { t } = useI18n()

function actionLabel(action: string) {
  const map: Record<string, string> = {
    allow: t('firewall.allow'),
    deny: t('firewall.deny'),
    reject: t('firewall.reject'),
  }
  return map[action] || action
}
</script>

<style scoped>
.row-actions {
  display: flex;
  gap: var(--fp-space-2);
}
.mono {
  font-family: var(--fp-font-mono);
  font-size: 12px;
}
</style>
