<template>
  <FpModal v-model="visible" :header="t('project.webhookTitle')">
    <div class="modal-form">
      <FpInlineMessage severity="info">{{ t('project.webhookOnceHint') }}</FpInlineMessage>
      <FpInput :model-value="token" :label="t('project.webhookToken')" readonly />
      <FpInput :model-value="url" :label="t('project.webhookUrl')" readonly />
      <FpInlineMessage severity="warn">{{ t('project.webhookPushHint') }}</FpInlineMessage>
    </div>
    <template #footer>
      <FpButton variant="ghost" @click="visible = false">{{ t('common.close') }}</FpButton>
      <FpButton variant="primary" @click="emit('copy')">{{ t('common.copy') }}</FpButton>
    </template>
  </FpModal>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpInlineMessage from '@/components/ui/FpInlineMessage.vue'

/**
 * Webhook 部署令牌弹窗（Doc/23 §3.3 T2-4 视图拆分第六站）。
 *
 * 令牌**仅本次响应返回**（后端不回显，重复开启即轮换），故弹窗做三件事：
 *
 * 1. 顶部提示「仅本次展示，请立即保存」——用户关掉就再也拿不到；
 * 2. 令牌与地址均为 `readonly` 输入框（可选中，但不可改——它是服务端事实）；
 * 3. 底部「复制」把**绝对地址**写进剪贴板（相对路径配到 CI 里连不回来，
 *    拼接与复制动作都在数据层，本组件只上抛 `copy`）。
 *
 * 令牌走 props 而非 `v-model`：它是**只读**的服务端事实，展示层没有写回路径。
 * 唯一可写的是可见性——数据层拿到开启结果后直接置位（令牌是新值，父级不可能
 * 「先开窗再填值」，那样会先闪一屏上一次的令牌）。
 */
withDefaults(
  defineProps<{
    /** 令牌明文（仅开启/轮换的那一次响应有值） */
    token?: string
    /** 部署端点（相对路径；复制时由数据层补 origin） */
    url?: string
  }>(),
  { token: '', url: '' },
)

const visible = defineModel<boolean>({ required: true })

const emit = defineEmits<{ copy: [] }>()

const { t } = useI18n()
</script>
