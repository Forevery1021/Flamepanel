<script setup lang="ts">
import { ref, nextTick, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Promotion, Delete, Plus, Search, Tools } from '@element-plus/icons-vue'
import api from '@/api/client'
import { useAuthStore } from '@/stores/auth'
import type { AiConversation, AiMessage, AiModelInfo, ToolInfo, ToolCallRequest } from '@/types'

const auth = useAuthStore()
const conversations = ref<AiConversation[]>([])
const models = ref<AiModelInfo[]>([])
const activeConvId = ref<number | null>(null)
const messages = ref<AiMessage[]>([])
const input = ref('')
const loading = ref(false)
const currentModel = ref('llama3')
const showSidebar = ref(true)

// Log analysis
const analyzeTab = ref<'chat' | 'analyze' | 'skills'>('chat')
const logContent = ref('')
const logResult = ref('')
const logAnalyzing = ref(false)

// Skills / Tools
const tools = ref<ToolInfo[]>([])
const toolCommand = ref('')
const toolArgs = ref('')
const toolResult = ref('')
const toolExecuting = ref(false)

const fetchTools = async () => {
  try {
    const res = await api.get<ToolInfo[]>('/ai/tools')
    tools.value = res.data
  } catch {
    // tools may not be available
  }
}

const executeTool = async (toolName: string, args?: Record<string, any>) => {
  toolExecuting.value = true
  toolResult.value = ''
  try {
    const res = await api.post('/ai/tools/call', {
      name: toolName,
      arguments: args || {},
    } as ToolCallRequest)
    toolResult.value = res.data.result
  } catch (e: any) {
    toolResult.value = `执行失败: ${e.response?.data?.message || e.message}`
  } finally {
    toolExecuting.value = false
  }
}

const executeCustomTool = async () => {
  let args: Record<string, any> | undefined
  if (toolArgs.value.trim()) {
    try {
      args = JSON.parse(toolArgs.value)
    } catch {
      ElMessage.warning('参数不是有效的 JSON')
      return
    }
  }
  await executeTool(toolCommand.value, args)
}

const fetchConversations = async () => {
  try {
    const res = await api.get<AiConversation[]>('/ai/conversations')
    conversations.value = res.data
  } catch {
    // Ollama may not be running
  }
}

const fetchModels = async () => {
  try {
    const res = await api.get<AiModelInfo[]>('/ai/models')
    models.value = res.data
    if (res.data.length > 0) {
      currentModel.value = res.data[0].name
    }
  } catch {
    // Ollama may not be running
  }
}

const selectConversation = async (conv: AiConversation) => {
  activeConvId.value = conv.id
  try {
    const parsed = JSON.parse(conv.messages) as AiMessage[]
    messages.value = parsed
  } catch {
    messages.value = []
  }
  await scrollBottom()
}

const newChat = async () => {
  activeConvId.value = null
  messages.value = []
  input.value = ''
}

const handleSend = async () => {
  const text = input.value.trim()
  if (!text || loading.value) return

  // Add user message immediately
  messages.value.push({ role: 'user', content: text })
  input.value = ''
  await scrollBottom()

  // Add empty assistant message for streaming
  messages.value.push({ role: 'assistant', content: '' })
  const assistantIdx = messages.value.length - 1
  loading.value = true

  try {
    const response = await fetch('/api/ai/chat/stream', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${auth.token}`,
      },
      body: JSON.stringify({
        conversation_id: activeConvId.value || undefined,
        model: currentModel.value,
        message: text,
      }),
    })

    if (!response.ok) {
      const err = await response.json().catch(() => ({ message: '请求失败' }))
      throw new Error(err.message || `HTTP ${response.status}`)
    }

    const reader = response.body!.getReader()
    const decoder = new TextDecoder()
    let buffer = ''

    while (true) {
      const { done, value } = await reader.read()
      if (done) break
      buffer += decoder.decode(value, { stream: true })

      // Parse SSE events (lines ending with \n)
      const lines = buffer.split('\n')
      buffer = lines.pop() || ''

      for (const line of lines) {
        if (line.startsWith('data: ')) {
          try {
            const data = JSON.parse(line.slice(6))
            if (data.token) {
              messages.value[assistantIdx].content += data.token
            } else if (data.done) {
              if (!activeConvId.value) {
                activeConvId.value = data.conversation_id
              }
              await fetchConversations()
            } else if (data.error) {
              messages.value[assistantIdx].content = `[错误] ${data.error}`
            }
          } catch {
            // skip malformed JSON
          }
        }
      }
      await scrollBottom()
    }
  } catch (e: any) {
    const errMsg = e.message || 'AI 请求失败，请确保 Ollama 正在运行'
    ElMessage.error(errMsg)
    messages.value[assistantIdx].content = `[错误] ${errMsg}`
  } finally {
    loading.value = false
  }
}

const handleDeleteConv = async (conv: AiConversation) => {
  try {
    await ElMessageBox.confirm(`确定删除对话「${conv.title}」？`, '确认', { type: 'warning' })
  } catch { return }
  try {
    await api.delete(`/ai/conversations/${conv.id}`)
    ElMessage.success('已删除')
    if (activeConvId.value === conv.id) {
      newChat()
    }
    await fetchConversations()
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '删除失败')
  }
}

const handleAnalyzeLogs = async () => {
  if (!logContent.value.trim()) {
    ElMessage.warning('请输入日志内容')
    return
  }
  logAnalyzing.value = true
  logResult.value = ''
  try {
    const res = await api.post('/ai/analyze', {
      log_content: logContent.value,
      model: currentModel.value || undefined,
    })
    logResult.value = res.data.analysis
  } catch (e: any) {
    logResult.value = `分析失败: ${e.response?.data?.message || '请确保 Ollama 正在运行且有可用模型'}`
  } finally {
    logAnalyzing.value = false
  }
}

const scrollBottom = async () => {
  await nextTick()
  const el = document.getElementById('chat-messages')
  if (el) el.scrollTop = el.scrollHeight
}

const formatDate = (s: string) => {
  if (!s) return ''
  return s.replace('T', ' ').substring(0, 16)
}

onMounted(() => {
  fetchConversations()
  fetchModels()
  fetchTools()
})
</script>

<template>
  <div class="ai-page">
    <!-- Sidebar -->
    <div class="ai-sidebar" :class="{ collapsed: !showSidebar }">
      <div class="sidebar-header">
        <el-button type="primary" size="small" @click="newChat">
          <el-icon><Plus /></el-icon> 新对话
        </el-button>
        <el-button size="small" text @click="showSidebar = !showSidebar" class="toggle-btn">
          {{ showSidebar ? '←' : '→' }}
        </el-button>
      </div>

      <div class="model-select">
        <span class="label">模型</span>
        <el-select v-model="currentModel" size="small" style="width: 100%" placeholder="无可用模型">
          <el-option
            v-for="m in models"
            :key="m.name"
            :label="`${m.name} (${m.size})`"
            :value="m.name"
          />
        </el-select>
      </div>

      <div class="conv-list">
        <div
          v-for="conv in conversations"
          :key="conv.id"
          class="conv-item"
          :class="{ active: activeConvId === conv.id }"
          @click="selectConversation(conv)"
        >
          <div class="conv-title">{{ conv.title }}</div>
          <div class="conv-meta">{{ conv.model }} · {{ formatDate(conv.updated_at) }}</div>
          <el-button
            class="conv-delete"
            size="small"
            text
            type="danger"
            @click.stop="handleDeleteConv(conv)"
          >
            <el-icon><Delete /></el-icon>
          </el-button>
        </div>
        <el-empty v-if="conversations.length === 0" description="暂无对话" :image-size="60" />
      </div>
    </div>

    <!-- Main -->
    <div class="ai-main">
      <div class="main-tabs">
        <div
          class="tab"
          :class="{ active: analyzeTab === 'chat' }"
          @click="analyzeTab = 'chat'"
        >
          <el-icon><Promotion /></el-icon> AI 对话
        </div>
        <div
          class="tab"
          :class="{ active: analyzeTab === 'analyze' }"
          @click="analyzeTab = 'analyze'"
        >
          <el-icon><Search /></el-icon> 日志分析
        </div>
        <div
          class="tab"
          :class="{ active: analyzeTab === 'skills' }"
          @click="analyzeTab = 'skills'"
        >
          <el-icon><Tools /></el-icon> Skills 工具
        </div>
      </div>

      <!-- Chat mode -->
      <div v-if="analyzeTab === 'chat'" class="chat-container">
        <div id="chat-messages" class="chat-messages">
          <div v-if="messages.length === 0" class="chat-welcome">
            <div class="welcome-icon">🤖</div>
            <h3>AI 运维助手</h3>
            <p>基于 Ollama 本地大模型，帮助您分析日志、排查故障、生成命令</p>
            <div class="quick-prompts">
              <div class="prompt-tag" @click="input = '当前服务器 CPU 使用率过高，可能是什么原因？如何排查？'">
                如何排查 CPU 过高？
              </div>
              <div class="prompt-tag" @click="input = '如何优化 Nginx 配置以提高并发性能？'">
                如何优化 Nginx 性能？
              </div>
              <div class="prompt-tag" @click="input = 'Docker 容器突然停止运行，如何快速定位问题？'">
                Docker 容器故障排查
              </div>
            </div>
          </div>

          <div
            v-for="(msg, i) in messages"
            :key="i"
            class="msg-row"
            :class="msg.role"
          >
            <template v-if="msg.content">
              <div class="msg-avatar">{{ msg.role === 'user' ? '👤' : '🤖' }}</div>
              <div class="msg-bubble" v-text="msg.content" />
            </template>
            <template v-else-if="loading && i === messages.length - 1">
              <div class="msg-avatar">🤖</div>
              <div class="msg-bubble typing">思考中...</div>
            </template>
          </div>
        </div>

        <div class="chat-input">
          <el-input
            v-model="input"
            type="textarea"
            :rows="3"
            placeholder="输入您的问题，AI 助手将为您解答..."
            @keydown.enter.exact.prevent="handleSend"
            :disabled="loading"
            resize="none"
          />
          <el-button
            type="primary"
            :loading="loading"
            @click="handleSend"
            style="margin-left: 10px; align-self: flex-end"
          >
            发送
          </el-button>
        </div>
      </div>

      <!-- Analyze mode -->
      <div v-if="analyzeTab === 'analyze'" class="analyze-container">
        <div class="analyze-input">
          <h4>粘贴日志内容</h4>
          <el-input
            v-model="logContent"
            type="textarea"
            :rows="10"
            placeholder="请粘贴需要分析的服务器日志、错误日志、Docker 日志等..."
            resize="vertical"
          />
          <el-button
            type="primary"
            :loading="logAnalyzing"
            @click="handleAnalyzeLogs"
            style="margin-top: 12px"
          >
            开始分析
          </el-button>
        </div>
        <div v-if="logResult" class="analyze-result">
          <h4>AI 分析结果</h4>
          <div class="result-content" v-text="logResult" />
        </div>
        <el-empty v-if="!logResult" description="AI 将自动识别日志中的异常、错误模式和安全威胁" :image-size="80" />
      </div>

      <!-- Skills mode -->
      <div v-if="analyzeTab === 'skills'" class="skills-container">
        <div class="skills-side">
          <h4>可用工具 ({{ tools.length }})</h4>
          <div class="tool-list">
            <div
              v-for="tool in tools"
              :key="tool.name"
              class="tool-card"
              @click="executeTool(tool.name)"
            >
              <div class="tool-name">{{ tool.name }}</div>
              <div class="tool-desc">{{ tool.description }}</div>
            </div>
            <el-empty v-if="tools.length === 0" description="暂无可用工具" :image-size="60" />
          </div>

          <h4 style="margin-top: 16px">自定义调用</h4>
          <div class="custom-call">
            <el-input v-model="toolCommand" placeholder="工具名称" size="small" />
            <el-input
              v-model="toolArgs"
              type="textarea"
              :rows="3"
              placeholder='参数 JSON（可选）如: {"path": "/var/log"}'
              size="small"
              style="margin-top: 6px"
            />
            <el-button
              type="primary"
              size="small"
              :loading="toolExecuting"
              :disabled="!toolCommand.trim()"
              @click="executeCustomTool"
              style="margin-top: 6px"
            >
              执行
            </el-button>
          </div>
        </div>

        <div class="skills-result">
          <h4>执行结果</h4>
          <div v-if="toolExecuting" class="result-loading">
            <el-icon class="is-loading"><Tools /></el-icon> 执行中...
          </div>
          <div v-else-if="toolResult" class="result-content" v-text="toolResult" />
          <el-empty v-else description="选择一个工具或输入命令来执行" :image-size="60" />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.ai-page {
  display: flex;
  height: calc(100vh - 60px);
  background: var(--bg-page);
}

/* Sidebar */
.ai-sidebar {
  width: 280px;
  min-width: 280px;
  border-right: 1px solid var(--border-color);
  background: var(--bg-card);
  display: flex;
  flex-direction: column;
  transition: width 0.3s, min-width 0.3s;
}
.ai-sidebar.collapsed {
  width: 0;
  min-width: 0;
  overflow: hidden;
  border: none;
}

.sidebar-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 14px;
  border-bottom: 1px solid var(--border-color);
}
.toggle-btn {
  margin-left: auto;
  font-size: 14px;
}

.model-select {
  padding: 12px 14px;
  border-bottom: 1px solid var(--border-color);
}
.model-select .label {
  font-size: 12px;
  color: var(--text-secondary);
  display: block;
  margin-bottom: 6px;
}

.conv-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}
.conv-item {
  padding: 10px 12px;
  border-radius: 8px;
  cursor: pointer;
  position: relative;
  margin-bottom: 4px;
  transition: background 0.15s;
}
.conv-item:hover {
  background: var(--bg-hover);
}
.conv-item.active {
  background: rgba(64, 158, 255, 0.12);
}
.conv-title {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  padding-right: 30px;
}
.conv-meta {
  font-size: 11px;
  color: var(--text-secondary);
  margin-top: 3px;
}
.conv-delete {
  position: absolute;
  right: 4px;
  top: 50%;
  transform: translateY(-50%);
  opacity: 0;
  transition: opacity 0.15s;
}
.conv-item:hover .conv-delete {
  opacity: 1;
}

/* Main */
.ai-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.main-tabs {
  display: flex;
  gap: 0;
  border-bottom: 1px solid var(--border-color);
  padding: 0 20px;
  background: var(--bg-card);
}
.tab {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 12px 20px;
  cursor: pointer;
  font-size: 14px;
  color: var(--text-secondary);
  border-bottom: 2px solid transparent;
  transition: all 0.2s;
}
.tab:hover {
  color: var(--text-primary);
}
.tab.active {
  color: #409eff;
  border-bottom-color: #409eff;
}

/* Chat */
.chat-container {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.chat-messages {
  flex: 1;
  overflow-y: auto;
  padding: 20px 24px;
}

.chat-welcome {
  text-align: center;
  padding: 40px 20px;
}
.welcome-icon {
  font-size: 52px;
  margin-bottom: 12px;
}
.chat-welcome h3 {
  margin: 0 0 8px;
  color: var(--text-primary);
}
.chat-welcome p {
  color: var(--text-secondary);
  font-size: 13px;
  margin-bottom: 20px;
}
.quick-prompts {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  justify-content: center;
}
.prompt-tag {
  padding: 6px 14px;
  border-radius: 16px;
  background: var(--bg-hover);
  color: #409eff;
  font-size: 12px;
  cursor: pointer;
  transition: background 0.2s;
}
.prompt-tag:hover {
  background: rgba(64, 158, 255, 0.15);
}

.msg-row {
  display: flex;
  gap: 10px;
  margin-bottom: 18px;
}
.msg-row.user {
  flex-direction: row-reverse;
}
.msg-avatar {
  width: 34px;
  height: 34px;
  border-radius: 50%;
  background: var(--bg-hover);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
  flex-shrink: 0;
}
.msg-bubble {
  max-width: 75%;
  padding: 10px 16px;
  border-radius: 12px;
  font-size: 13px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-word;
}
.msg-row.user .msg-bubble {
  background: #409eff;
  color: white;
  border-bottom-right-radius: 4px;
}
.msg-row.assistant .msg-bubble {
  background: var(--bg-card);
  color: var(--text-primary);
  border: 1px solid var(--border-color);
  border-bottom-left-radius: 4px;
}
.msg-bubble.typing {
  color: var(--text-secondary);
  font-style: italic;
  padding: 10px 20px;
}

.chat-input {
  display: flex;
  padding: 14px 24px;
  border-top: 1px solid var(--border-color);
  background: var(--bg-card);
  gap: 10px;
}

/* Analyze */
.analyze-container {
  flex: 1;
  overflow-y: auto;
  padding: 20px 24px;
}
.analyze-input h4,
.analyze-result h4 {
  margin: 0 0 10px;
  color: var(--text-primary);
}
.analyze-result {
  margin-top: 20px;
}
.result-content {
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  padding: 16px;
  font-size: 13px;
  line-height: 1.7;
  white-space: pre-wrap;
  word-break: break-word;
  color: var(--text-primary);
}

/* Skills */
.skills-container {
  flex: 1;
  display: flex;
  min-height: 0;
}
.skills-side {
  width: 340px;
  min-width: 340px;
  border-right: 1px solid var(--border-color);
  padding: 16px;
  overflow-y: auto;
}
.skills-side h4 {
  margin: 0 0 10px;
  color: var(--text-primary);
  font-size: 13px;
}
.tool-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.tool-card {
  padding: 10px 12px;
  border-radius: 8px;
  border: 1px solid var(--border-color);
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s;
}
.tool-card:hover {
  background: var(--bg-hover);
  border-color: #409eff;
}
.tool-name {
  font-size: 12px;
  font-weight: 600;
  color: #409eff;
  font-family: monospace;
  margin-bottom: 3px;
}
.tool-desc {
  font-size: 11px;
  color: var(--text-secondary);
  line-height: 1.5;
}
.custom-call {
  display: flex;
  flex-direction: column;
}
.skills-result {
  flex: 1;
  padding: 16px;
  overflow-y: auto;
}
.skills-result h4 {
  margin: 0 0 10px;
  color: var(--text-primary);
}
.result-loading {
  color: var(--text-secondary);
  font-size: 13px;
  display: flex;
  align-items: center;
  gap: 6px;
}
</style>
