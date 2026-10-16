<template>
  <div class="modal-form">
    <FpInput v-model="model.name" :label="t('website.name')" :error="errors.name" />
    <FpInput v-model="model.domain" :label="t('website.domain')" :error="errors.domain" />
    <FpInput v-model="model.root_path" :label="t('website.rootPath')" />
    <FpSelect
      v-model="model.site_type"
      :label="t('website.siteType')"
      :options="siteTypeOptions"
      option-label="label"
      option-value="value"
    />
    <div class="field-col">
      <label class="field-label">{{ t('website.nodeId') }}</label>
      <FpNumber v-model="model.node_id" :min="1" class="w-full" />
    </div>
    <FpSelect
      v-model="model.engine"
      :label="t('website.engine')"
      :options="engineOptions"
      option-label="label"
      option-value="value"
    />
    <FpSwitch v-model="model.ssl_enabled" :label="t('website.sslEnabled')" />
    <FpSwitch v-model="model.proxy_enabled" :label="t('website.proxyEnabled')" />
    <FpInput
      v-if="model.proxy_enabled"
      v-model="model.proxy_pass"
      :label="t('website.proxyPass')"
    />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { siteTypeOptions as buildSiteTypeOptions } from '@/utils/websiteFormat'
import type { WebsiteFormPayload } from '@/composables/useWebsiteList'
import FpInput from '@/components/ui/FpInput.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpNumber from '@/components/ui/FpNumber.vue'
import FpSwitch from '@/components/ui/FpSwitch.vue'

/**
 * 新建 / 编辑网站的表单字段（Doc/23 §3.3 T2-4 视图拆分第二站）。
 *
 * 两处弹窗字段完全一致（差异只在标题与提交目标），故抽为单一组件、以 `v-model`
 * 双向绑定表单对象——**编辑态由父级 `WebsiteDialogs` 本地持有**，本组件不复制副本，
 * 避免「改一处漏一处」的表单漂移。
 *
 * 站点类型下拉的候选清单取自 `SITE_TYPES`（与后端白名单对齐的单一事实来源），
 * 文案按当前语言现算（切换语言即时更新）。
 */
const model = defineModel<WebsiteFormPayload>({ required: true })

withDefaults(
  defineProps<{
    errors?: { name: string; domain: string }
    /** 引擎候选（与切换引擎弹窗共用同一份静态清单） */
    engineOptions?: { label: string; value: string }[]
  }>(),
  { errors: () => ({ name: '', domain: '' }), engineOptions: () => [] },
)

const { t } = useI18n()
const siteTypeOptions = computed(() => buildSiteTypeOptions(t))
</script>
