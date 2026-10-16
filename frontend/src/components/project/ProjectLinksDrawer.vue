<template>
  <FpDrawer v-model="visible" :header="t('project.linksTitle', { name: project?.name ?? '' })">
    <div class="links-body">
      <!-- 已关联列表 -->
      <!--
        ⚠️ 这里的 empty-text 是**原实现的原样保留**：FpStatePanel 只认
        `empty-title` / `empty-desc`，`empty-text` 会作为降级属性被静默丢弃，
        故空态实际渲染的是面板默认的「暂无数据」，`project.linksEmpty`
        （「尚未关联任何数据库实例」）从未上屏。修它属于可见文案变更，
        不在本轮「拆分零契约变更」的范围内——已记入 PR「后续建议」。
      -->
      <FpStatePanel
        :loading="loading"
        :error="error"
        :empty="links.length === 0"
        :empty-text="t('project.linksEmpty')"
        retryable
        @retry="emit('refresh')"
      >
        <div v-for="link in links" :key="link.id" class="link-row">
          <div class="link-row__info">
            <FpTag
              :value="link.db_type"
              :severity="link.db_type === 'unknown' ? 'danger' : 'info'"
            />
            <span class="link-row__name">{{ link.db_instance_name }}</span>
            <code class="link-row__env">{{ link.env_name }}</code>
          </div>
          <FpButton
            v-permission="{ perm: 'project:update', mode: 'view' }"
            variant="danger"
            size="small"
            :loading="actingId === `unlink-${link.id}`"
            @click="emit('unlink', link)"
          >
            {{ t('project.linkUnlink') }}
          </FpButton>
        </div>
      </FpStatePanel>

      <FpInlineMessage severity="info">{{ t('project.linksHint') }}</FpInlineMessage>

      <!-- 关联表单 -->
      <div class="link-form">
        <FpSelect
          :model-value="form.db_instance_id"
          :label="t('project.linkInstance')"
          :options="linkableOptions"
          option-label="label"
          option-value="value"
          show-clear
          placeholder=""
          @update:model-value="(v) => emit('update:instanceId', v as number | undefined)"
        />
        <FpInput
          :model-value="form.env_name"
          :label="t('project.linkEnvName')"
          :placeholder="t('project.linkEnvNamePlaceholder')"
          @update:model-value="(v) => emit('update:envName', v)"
        />
        <FpButton
          v-permission="{ perm: 'project:update', mode: 'view' }"
          variant="primary"
          :disabled="!form.db_instance_id"
          :loading="saving"
          @click="emit('link')"
        >
          {{ t('project.linkCreate') }}
        </FpButton>
      </div>
    </div>
  </FpDrawer>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { ProjectDbLinkDto, ProjectDto } from '@/api/generated'
import FpDrawer from '@/components/ui/FpDrawer.vue'
import FpStatePanel from '@/components/ui/FpStatePanel.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpTag from '@/components/ui/FpTag.vue'
import FpInlineMessage from '@/components/ui/FpInlineMessage.vue'

/**
 * 数据库联链抽屉（Doc/23 §3.3 T2-4 视图拆分第六站；Stage A-10 · Doc/20 §5.8 ⑤）。
 *
 * 两条容易在拆分中被抹平的细节：
 *
 * 1. **`unknown` 类型的实例标红**（`:severity="link.db_type === 'unknown' ? 'danger' : 'info'"`）
 *    ——连上的实例探测不出数据库类型，多半是容器刚起或已挂；染成中性色等于放过一个
 *    真实的故障信号。这是原实现里唯一一处红色标签，拆分后原样保留。
 * 2. **表单两字段走上抛而非 `v-model` 双向绑**：编辑态由数据层持有
 *    （`useProjectLinks.form`），组件只负责渲染与把每字段的新值抛回去——
 *    两个字段的写入路径合成一条「数据层唯一持有」的线，抽屉打开时清空也只需
 *    在数据层做一次（打开另一个项目不会残留上次填的变量名）。
 */
withDefaults(
  defineProps<{
    /** 当前查看的项目（仅用于标题） */
    project?: ProjectDto | null
    /** 已关联列表（后端不回连接串） */
    links?: ProjectDbLinkDto[]
    /** 可关联实例候选（后端只返回 running 实例） */
    linkableOptions?: { label: string; value: number }[]
    /** 关联表单（编辑态在数据层） */
    form?: { db_instance_id: number | undefined; env_name: string }
    loading?: boolean
    error?: string | null
    saving?: boolean
    /** 正在进行的联链操作标签（格式 `unlink-<linkId>`；只锁该行按钮） */
    actingId?: string
  }>(),
  {
    project: null,
    links: () => [],
    linkableOptions: () => [],
    form: () => ({ db_instance_id: undefined, env_name: '' }),
    loading: false,
    error: null,
    saving: false,
    actingId: '',
  },
)

const visible = defineModel<boolean>({ required: true })

const emit = defineEmits<{
  refresh: []
  link: []
  unlink: [link: ProjectDbLinkDto]
  'update:instanceId': [value: number | undefined]
  'update:envName': [value: string | undefined]
}>()

const { t } = useI18n()
</script>

<style scoped>
.links-body {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-4);
}
.link-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--fp-space-2);
  padding: var(--fp-space-2) 0;
  border-bottom: 1px solid var(--fp-border);
}
.link-row__info {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
}
.link-row__name {
  font-weight: 600;
}
.link-row__env {
  font-family: var(--fp-font-mono);
  font-size: 12px;
  color: var(--fp-text-secondary);
}
.link-form {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-3);
}
</style>
