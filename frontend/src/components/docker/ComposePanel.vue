<template>
  <div class="panel">
    <div class="compose-toolbar">
      <FpButton
        v-permission="{ perm: 'docker:create', mode: 'view' }"
        variant="secondary"
        icon="oi oi-file-import"
        @click="emit('import-run')"
        >{{ t('docker.importRun') }}</FpButton
      >
    </div>
    <FpInlineMessage v-if="warnings.length" severity="warn" class="convert-warnings">
      <div class="warnings-title">{{ t('docker.convertWarnings') }}</div>
      <ul>
        <li v-for="w in warnings" :key="w">{{ w }}</li>
      </ul>
    </FpInlineMessage>
    <form class="modal-form" @submit.prevent="emit('deploy')">
      <FpInput
        v-model="name"
        :label="t('docker.projectName')"
        :placeholder="t('common.placeholder')"
      />
      <FpTextarea
        v-model="yaml"
        :label="t('docker.yaml')"
        :rows="12"
        class="compose-textarea w-full"
      />
      <div class="form-actions">
        <FpButton
          v-permission="{ perm: 'docker:create', mode: 'view' }"
          variant="primary"
          type="submit"
          icon="oi oi-upload"
          :loading="submitting"
          >{{ t('docker.deploy') }}</FpButton
        >
        <FpButton
          v-permission="{ perm: 'docker:start', mode: 'view' }"
          variant="ghost"
          :disabled="!name"
          @click="emit('up', name)"
          >{{ t('docker.up') }}</FpButton
        >
        <FpButton
          v-permission="{ perm: 'docker:stop', mode: 'view' }"
          variant="ghost"
          :disabled="!name"
          @click="emit('down', name)"
          >{{ t('docker.down') }}</FpButton
        >
      </div>
    </form>
    <FpDivider align="left">{{ t('docker.projects') }}</FpDivider>
    <FpTable
      :rows="rows"
      :loading="loading"
      :paginator="false"
      :empty-text="t('common.noData')"
      striped-rows
      virtual
      virtual-scroll-height="360px"
    >
      <FpColumn field="name" :header="t('docker.projectName')" style="min-width: 140px" />
      <FpColumn field="status" :header="t('docker.projectStatus')" style="width: 120px" />
      <FpColumn
        field="config_files"
        :header="t('docker.projectConfigFiles')"
        style="min-width: 200px"
      />
      <FpColumn :header="t('docker.actions')" style="width: 180px" frozen align-frozen="right">
        <template #body="{ data }">
          <div class="row-actions">
            <FpButton
              v-permission="{ perm: 'docker:start', mode: 'view' }"
              variant="ghost"
              @click="emit('up', data.name)"
              >{{ t('docker.up') }}</FpButton
            >
            <FpButton
              v-permission="{ perm: 'docker:stop', mode: 'view' }"
              variant="danger"
              @click="emit('down', data.name)"
              >{{ t('docker.down') }}</FpButton
            >
          </div>
        </template>
      </FpColumn>
    </FpTable>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { ComposeProject } from '@/types'
import FpInput from '@/components/ui/FpInput.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpDivider from '@/components/ui/FpDivider.vue'
import FpTable from '@/components/ui/FpTable.vue'
import FpTextarea from '@/components/ui/FpTextarea.vue'
import FpInlineMessage from '@/components/ui/FpInlineMessage.vue'

/**
 * Docker Compose 页签（Doc/23 §3.3 T2-4 视图拆分第三站）。
 *
 * 上半是部署表单、下半是已部署项目表——两者共用同一个 `name`：
 * 表单里填的项目名既是「部署」的输入，也是「启动/停止」（up/down）的目标，
 * 因此 up/down 按钮的禁用条件就是 `!name`。这个耦合是原实现的设计（不是缺陷），
 * 拆出后由同一个 `v-model:name` 承载，语义比原先散在视图里更显式。
 *
 * 项目表的两个行内动作作用于**行内项目名**，与表单输入无关——两条 up/down
 * 路径（表单 / 行内）是各自独立的上抛，用例分别锁死。
 */
withDefaults(
  defineProps<{
    rows: ComposeProject[]
    loading?: boolean
    submitting?: boolean
    /** 最近一次转换（docker run 导入 / 容器收编）的提示；表单被回填时由父级传入 */
    warnings?: string[]
  }>(),
  { loading: false, submitting: false, warnings: () => [] },
)

const name = defineModel<string>('name', { default: '' })
const yaml = defineModel<string>('yaml', { default: '' })

const emit = defineEmits<{
  deploy: []
  'import-run': []
  up: [name: string]
  down: [name: string]
}>()

const { t } = useI18n()
</script>

<style scoped>
.form-actions {
  display: flex;
  gap: var(--fp-space-2);
}
.compose-toolbar {
  display: flex;
  justify-content: flex-end;
  margin-bottom: var(--fp-space-2);
}
.convert-warnings {
  margin-bottom: var(--fp-space-3);
}
.convert-warnings ul {
  margin: var(--fp-space-1) 0 0;
  padding-left: 1.2em;
}
.warnings-title {
  font-weight: 600;
}
.compose-textarea {
  font-family: var(--fp-font-mono);
}
.row-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
</style>
