/**
 * 面板内置 AI 助手 API（M-AI-3）。类型来自 OpenAPI 生成（src/api/generated）。
 */
import api from './client'
import type {
  AssistantChatResponse,
  AssistantChatTurn,
  AssistantConfigPatch,
  AssistantConfigView,
  AssistantTestResult,
  AssistantToolStep,
} from '@/api/generated'

export type {
  AssistantChatResponse,
  AssistantChatTurn,
  AssistantConfigPatch,
  AssistantConfigView,
  AssistantTestResult,
  AssistantToolStep,
}

/** 单次对话最长等待：模型多轮工具调用可能超过默认 15 s */
export const CHAT_TIMEOUT_MS = 10 * 60 * 1000

export function getAssistantConfig() {
  return api.get<AssistantConfigView>('/assistant/config')
}

export function updateAssistantConfig(patch: AssistantConfigPatch) {
  return api.put<AssistantConfigView>('/assistant/config', patch)
}

export function testAssistant() {
  return api.post<AssistantTestResult>('/assistant/test', undefined, { timeout: 5 * 60 * 1000 })
}

export function chatAssistant(messages: AssistantChatTurn[], conversationId?: string | null) {
  return api.post<AssistantChatResponse>(
    '/assistant/chat',
    { messages, conversation_id: conversationId ?? null },
    { timeout: CHAT_TIMEOUT_MS },
  )
}
