<template>
  <div class="panel assistant-chat">
    <div class="chat-head">
      <div>
        <h3 class="section-title">{{ t('assistant.chatTitle') }}</h3>
        <p class="muted">{{ t('assistant.intro') }}</p>
      </div>
      <FpButton
        v-if="messages.length"
        variant="ghost"
        size="small"
        icon="oi oi-plus"
        :label="t('assistant.newChat')"
        :disabled="sending"
        @click="reset"
      />
    </div>

    <FpEmpty v-if="!canChat" icon="oi oi-lock" :title="t('assistant.noChatPerm')" />
    <FpEmpty
      v-else-if="!enabled"
      icon="oi oi-sparkles"
      :title="t('assistant.disabledTitle')"
      :description="canConfigure ? t('assistant.disabledHint') : t('assistant.disabledNoPerm')"
    />
    <template v-else>
      <div ref="listEl" class="messages" role="log" aria-live="polite">
        <div v-for="(m, i) in messages" :key="i" class="msg" :class="m.role">
          <div class="who">{{ m.role === 'user' ? t('assistant.you') : t('assistant.bot') }}</div>
          <div class="bubble">
            <details v-if="m.steps?.length" class="steps">
              <summary>{{ t('assistant.steps', { n: m.steps.length }) }}</summary>
              <div v-for="(s, j) in m.steps" :key="j" class="step">
                <div class="step-head">
                  <span class="mono">{{ s.tool }}</span>
                  <FpTag v-if="!s.ok" severity="danger" :value="t('assistant.stepFailed')" />
                  <code class="step-args" :title="s.arguments">{{ s.arguments }}</code>
                </div>
                <pre class="step-output">{{ s.output }}</pre>
              </div>
            </details>
            <div class="content">{{ m.content }}</div>
            <div v-if="m.approvalIds?.length" class="approvals">
              <FpTag
                v-for="id in m.approvalIds"
                :key="id"
                severity="warning"
                :value="t('assistant.approvalCreated', { id })"
              />
              <FpButton
                variant="link"
                size="small"
                :label="t('assistant.goApprove')"
                @click="emit('open-approvals')"
              />
            </div>
            <FpInlineMessage v-if="m.stepLimit" severity="warn">{{
              t('assistant.stepLimit')
            }}</FpInlineMessage>
          </div>
        </div>
        <div v-if="sending" class="msg assistant pending">
          <div class="who">{{ t('assistant.bot') }}</div>
          <div class="bubble muted">{{ t('assistant.thinking') }}</div>
        </div>
      </div>

      <div v-if="runbooks?.length" class="runbooks">
        <span class="runbooks-label" :title="t('assistant.runbooksHint')">
          <i class="oi oi-sparkles" aria-hidden="true" /> {{ t('assistant.runbooks') }}
        </span>
        <FpButton
          v-for="rb in runbooks"
          :key="rb.name"
          size="small"
          variant="secondary"
          :label="rb.title"
          :title="rb.description"
          :disabled="sending || rendering"
          @click="pickRunbook(rb)"
        />
      </div>
      <form v-if="activeRunbook" class="runbook-form" @submit.prevent="applyRunbook">
        <p class="muted">{{ activeRunbook.description }}</p>
        <FpInput
          v-for="a in activeRunbook.arguments"
          :key="a.name"
          v-model="runbookArgs[a.name]"
          :label="a.required ? `${a.description} *` : a.description"
          maxlength="200"
        />
        <div class="runbook-actions">
          <FpButton
            variant="ghost"
            size="small"
            :label="t('common.cancel')"
            @click="activeRunbook = null"
          />
          <FpButton
            type="submit"
            size="small"
            :label="t('assistant.useRunbook')"
            :loading="rendering"
            :disabled="!runbookReady"
          />
        </div>
      </form>

      <form class="composer" @submit.prevent="send">
        <FpTextarea
          v-model="input"
          rows="3"
          maxlength="8000"
          :placeholder="t('assistant.placeholder')"
          :aria-label="t('assistant.placeholder')"
          :disabled="sending"
          @keydown="onKeydown"
        />
        <FpButton
          type="submit"
          icon="oi oi-send"
          :label="t('assistant.send')"
          :loading="sending"
          :disabled="!input.trim()"
        />
      </form>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import FpButton from '@/components/ui/FpButton.vue'
import FpEmpty from '@/components/ui/FpEmpty.vue'
import FpInlineMessage from '@/components/ui/FpInlineMessage.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpTag from '@/components/ui/FpTag.vue'
import FpTextarea from '@/components/ui/FpTextarea.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { chatAssistant, type AssistantToolStep } from '@/api/assistant'
import { renderMcpPrompt, type McpPromptInfo } from '@/api/mcp'

/**
 * 面板助手对话（M-AI-3）。对话状态只保存在本组件内存中（刷新即清空），
 * 每轮把 user / assistant 文本历史发给后端；工具轨迹仅用于展示，不回传。
 */
defineProps<{
  enabled: boolean
  canChat: boolean
  canConfigure: boolean
  /** 运维技能库（Prompts）；为空时不显示快捷入口 */
  runbooks?: McpPromptInfo[]
}>()
const emit = defineEmits<{ 'open-approvals': [] }>()

const { t } = useI18n()
const toast = useFpToast()

/** 与后端 MAX_HISTORY 一致 */
const MAX_HISTORY = 40

interface ChatEntry {
  role: 'user' | 'assistant'
  content: string
  steps?: AssistantToolStep[]
  approvalIds?: number[]
  stepLimit?: boolean
}

const messages = ref<ChatEntry[]>([])
const input = ref('')
const sending = ref(false)
const conversationId = ref<string | null>(null)
const listEl = ref<HTMLElement | null>(null)

async function scrollToBottom() {
  await nextTick()
  const el = listEl.value
  if (el) el.scrollTop = el.scrollHeight
}

function onKeydown(e: KeyboardEvent) {
  // Enter 发送，Shift+Enter 换行；输入法组字中不拦截
  if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) {
    e.preventDefault()
    void send()
  }
}

async function send() {
  const text = input.value.trim()
  if (!text || sending.value) return
  if (messages.value.length + 1 > MAX_HISTORY) {
    toast.warning(t('assistant.tooLong'))
    return
  }
  messages.value.push({ role: 'user', content: text })
  input.value = ''
  sending.value = true
  void scrollToBottom()
  try {
    const history = messages.value.map((m) => ({ role: m.role, content: m.content }))
    const res = await chatAssistant(history, conversationId.value)
    const r = res.data
    conversationId.value = r.conversation_id
    messages.value.push({
      role: 'assistant',
      content: r.reply,
      steps: r.steps,
      approvalIds: r.approval_ids,
      stepLimit: r.step_limit_reached,
    })
  } catch (err) {
    // 失败：撤回这条用户消息并放回输入框，便于修改后重发
    messages.value.pop()
    input.value = text
    toast.error(err, t('common.failed'))
  } finally {
    sending.value = false
    void scrollToBottom()
  }
}

// ── 运维技能：渲染后填入输入框，由用户确认再发送 ──
const activeRunbook = ref<McpPromptInfo | null>(null)
const runbookArgs = reactive<Record<string, string>>({})
const rendering = ref(false)
const runbookReady = computed(
  () =>
    !!activeRunbook.value?.arguments.every(
      (a) => !a.required || (runbookArgs[a.name] ?? '').trim() !== '',
    ),
)

async function pickRunbook(rb: McpPromptInfo) {
  for (const k of Object.keys(runbookArgs)) delete runbookArgs[k]
  for (const a of rb.arguments) runbookArgs[a.name] = ''
  activeRunbook.value = rb
  if (rb.arguments.length === 0) await applyRunbook()
}

async function applyRunbook() {
  const rb = activeRunbook.value
  if (!rb || !runbookReady.value) return
  rendering.value = true
  try {
    const res = await renderMcpPrompt(rb.name, { ...runbookArgs })
    input.value = res.data.text
    activeRunbook.value = null
  } catch (err) {
    toast.error(err, t('common.failed'))
  } finally {
    rendering.value = false
  }
}

function reset() {
  messages.value = []
  conversationId.value = null
  input.value = ''
  activeRunbook.value = null
}

defineExpose({ send, reset })
</script>

<style scoped>
.chat-head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: var(--fp-space-3);
}
.section-title {
  margin: 0 0 var(--fp-space-1);
  font-size: var(--fp-font-md, 15px);
}
.muted {
  color: var(--fp-text-muted);
  font-size: 13px;
  margin: 0 0 var(--fp-space-3);
}
.messages {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-3);
  max-height: 60vh;
  min-height: 160px;
  overflow-y: auto;
  padding: var(--fp-space-2);
  border: 1px solid var(--fp-border);
  border-radius: var(--fp-radius, 6px);
  margin-bottom: var(--fp-space-3);
}
.msg {
  display: flex;
  flex-direction: column;
  max-width: 85%;
}
.msg.user {
  align-self: flex-end;
  align-items: flex-end;
}
.who {
  font-size: 12px;
  color: var(--fp-text-muted);
  margin-bottom: 2px;
}
.bubble {
  padding: var(--fp-space-2) var(--fp-space-3);
  border-radius: var(--fp-radius, 6px);
  background: var(--fp-bg-subtle, rgba(127, 127, 127, 0.08));
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-2);
}
.msg.user .bubble {
  background: var(--fp-primary-soft, rgba(59, 130, 246, 0.12));
}
.content {
  white-space: pre-wrap;
  word-break: break-word;
  line-height: 1.6;
}
.steps summary {
  cursor: pointer;
  font-size: 12px;
  color: var(--fp-text-muted);
}
.step {
  margin-top: var(--fp-space-2);
}
.step-head {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
  font-size: 12px;
}
.step-args {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 360px;
  color: var(--fp-text-muted);
}
.step-output {
  margin: var(--fp-space-1) 0 0;
  max-height: 180px;
  overflow: auto;
  font-size: 12px;
  white-space: pre-wrap;
  word-break: break-all;
}
.approvals {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--fp-space-2);
}
.runbooks {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--fp-space-2);
  margin-bottom: var(--fp-space-2);
}
.runbooks-label {
  font-size: 12px;
  color: var(--fp-text-muted);
}
.runbook-form {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-2);
  padding: var(--fp-space-3);
  margin-bottom: var(--fp-space-2);
  border: 1px dashed var(--fp-border);
  border-radius: var(--fp-radius, 6px);
}
.runbook-form .muted {
  margin: 0;
}
.runbook-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--fp-space-2);
}
.composer {
  display: flex;
  gap: var(--fp-space-2);
  align-items: flex-end;
}
.composer > :first-child {
  flex: 1;
}
</style>
