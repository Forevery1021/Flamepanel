<template>
  <FpModal v-model="open" :header="`${t('plugin.capabilities')} — ${plugin?.id ?? ''}`" size="md">
    <div class="modal-form">
      <FpTextarea
        v-model="text"
        :label="t('plugin.capabilities')"
        :rows="5"
        class="w-full mono"
        placeholder="metrics:read"
      />
      <div class="hint">{{ t('plugin.capabilitiesHint') }}</div>
      <CapabilityTags :capabilities="parsed" detailed />
    </div>
    <template #footer>
      <FpButton variant="ghost" @click="open = false">{{ t('common.cancel') }}</FpButton>
      <FpButton variant="primary" :loading="saving" @click="save">{{ t('common.save') }}</FpButton>
    </template>
  </FpModal>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import FpModal from '@/components/ui/FpModal.vue'
import FpTextarea from '@/components/ui/FpTextarea.vue'
import FpButton from '@/components/ui/FpButton.vue'
import { useFpToast } from '@/components/ui/FpToast'
import CapabilityTags from './CapabilityTags.vue'
import { updatePluginCapabilities } from '@/api/plugins'
import { parseCapabilityInput } from '@/utils/pluginCapabilities'
import { getErrorMessage } from '@/utils/error'
import type { PluginResponse } from '@/types'

const props = defineProps<{ plugin: PluginResponse | null }>()
const emit = defineEmits<{ saved: [] }>()
const open = defineModel<boolean>({ default: false })
const { t } = useI18n()
const toast = useFpToast()

const text = ref('')
const saving = ref(false)
const parsed = computed(() => parseCapabilityInput(text.value))

watch(open, (v) => {
  if (v) text.value = (props.plugin?.capabilities ?? []).join('\n')
})

async function save() {
  if (!props.plugin) return
  saving.value = true
  try {
    await updatePluginCapabilities(props.plugin.id, parsed.value)
    toast.success(t('common.success'))
    open.value = false
    emit('saved')
  } catch (e: unknown) {
    toast.error(getErrorMessage(e, t('common.failed')))
  } finally {
    saving.value = false
  }
}
</script>

<style scoped>
.hint {
  font-size: 12px;
  color: var(--fp-text-muted);
}
</style>
