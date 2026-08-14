<template>
  <div class="memo-list">
    <div class="memo-toolbar">
      <FpInput
        v-model="newContent"
        :placeholder="t('memo.placeholder')"
        class="memo-input"
        @keyup.enter="add"
      />
      <FpButton variant="primary" :loading="adding" @click="add">{{ t('common.add') }}</FpButton>
      <label class="memo-filter">
        <FpCheckbox v-model="showDone" size="small" />
        {{ t('memo.showDone') }}
      </label>
    </div>

    <div v-if="loading" class="memo-skeleton">
      <FpSkeleton v-for="i in 5" :key="i" height="36px" />
    </div>
    <div v-else class="memo-items">
      <div v-for="m in items" :key="m.id" class="memo-item" :class="{ done: m.done }">
        <FpCheckbox :model-value="m.done" @change="(v) => toggle(m, Boolean(v))" />
        <span class="memo-content" @dblclick="startEdit(m)">{{ m.content }}</span>
        <span class="memo-time mono">{{ shortTime(m.created_at) }}</span>
        <FpButton variant="link" icon="oi oi-pencil" :aria-label="t('memo.edit')" @click="startEdit(m)" />
        <FpButton
          variant="link"
          icon="oi oi-trash"
          class="danger-link"
          :aria-label="t('common.delete')"
          @click="confirmDelete(m)"
        />
      </div>
      <div v-if="!items.length" class="memo-empty">{{ t('common.noData') }}</div>
    </div>

    <FpModal v-model="editVisible" :header="t('memo.edit')">
      <FpTextarea v-model="editContent" :rows="4" class="w-full" />
      <template #footer>
        <FpButton variant="ghost" @click="editVisible = false">{{ t('common.cancel') }}</FpButton>
        <FpButton variant="primary" @click="saveEdit">{{ t('common.save') }}</FpButton>
      </template>
    </FpModal>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'



import { listMemos, createMemo, updateMemo, deleteMemo } from '@/api/memos'
import FpInput from '@/components/ui/FpInput.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpModal from '@/components/ui/FpModal.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import FpCheckbox from '@/components/ui/FpCheckbox.vue'
import FpSkeleton from '@/components/ui/FpSkeleton.vue'
import FpTextarea from '@/components/ui/FpTextarea.vue'
import type { Memo } from '@/types'

const props = defineProps<{ kind: string }>()
const { t } = useI18n()
const toast = useFpToast()
const { confirmAction } = useFpConfirm()

const items = ref<Memo[]>([])
const loading = ref(false)
const adding = ref(false)
const newContent = ref('')
const showDone = ref(true)

const editVisible = ref(false)
const editContent = ref('')
let editingId = 0

async function fetch() {
  loading.value = true
  try {
    const res = await listMemos(props.kind, showDone.value ? undefined : false)
    items.value = res.data
  } catch {
    toast.error(t('common.failed'))
  } finally {
    loading.value = false
  }
}

async function add() {
  if (!newContent.value.trim()) return
  adding.value = true
  try {
    await createMemo(newContent.value.trim(), props.kind)
    newContent.value = ''
    await fetch()
  } catch {
    toast.error(t('common.failed'))
  } finally {
    adding.value = false
  }
}

async function toggle(m: Memo, done: boolean) {
  try {
    await updateMemo(m.id, { done })
    m.done = done
  } catch {
    toast.error(t('common.failed'))
  }
}

function startEdit(m: Memo) {
  editingId = m.id
  editContent.value = m.content
  editVisible.value = true
}

async function saveEdit() {
  if (!editContent.value.trim()) return
  try {
    await updateMemo(editingId, { content: editContent.value.trim() })
    editVisible.value = false
    await fetch()
  } catch {
    toast.error(t('common.failed'))
  }
}

function confirmDelete(m: Memo) {
  confirmAction({
    message: t('memo.deleteConfirm'),
    accept: async () => {
      try {
        await deleteMemo(m.id)
        await fetch()
      } catch {
        toast.error(t('common.failed'))
      }
    },
  })
}

function shortTime(ts: string) {
  const d = new Date(ts)
  return `${d.getMonth() + 1}/${d.getDate()} ${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`
}

watch(showDone, fetch)
onMounted(fetch)
</script>

<style scoped>
.memo-toolbar {
  display: flex;
  align-items: center;
  gap: var(--fp-space-3);
  margin-bottom: var(--fp-space-4);
  flex-wrap: wrap;
}
.memo-input {
  max-width: 420px;
}
.memo-filter {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  color: var(--fp-text-secondary);
  cursor: pointer;
  margin-left: auto;
}
.memo-skeleton {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-2);
  min-height: 200px;
}
.memo-items {
  min-height: 200px;
}
.memo-item {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
  padding: var(--fp-space-2) var(--fp-space-3);
  border-radius: var(--fp-radius-sm);
  border-bottom: 1px solid var(--fp-border);
}
.memo-item:hover {
  background: var(--fp-bg-hover);
}
.memo-item.done .memo-content {
  text-decoration: line-through;
  color: var(--fp-text-muted);
}
.memo-content {
  flex: 1;
  font-size: 14px;
  color: var(--fp-text-primary);
  cursor: text;
  word-break: break-all;
}
.memo-time {
  font-size: 12px;
  color: var(--fp-text-muted);
  flex-shrink: 0;
}
.memo-empty {
  padding: 40px;
  text-align: center;
  color: var(--fp-text-muted);
}
.danger-link {
  color: var(--fp-text-muted);
}
</style>
