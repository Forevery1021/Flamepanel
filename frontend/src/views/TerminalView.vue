<template>
  <div class="view-container terminal-page">
    <div class="panel terminal-card">
      <div class="card-header-title">
        <span>{{
          nodeId ? t('terminal.remoteTitle', { name: nodeName }) : t('terminal.title')
        }}</span>
        <div class="header-right">
          <FpTag
            :severity="connected ? 'success' : 'danger'"
            :value="connected ? t('terminal.connected') : t('terminal.disconnected')"
          />
          <FpButton variant="ghost" :disabled="!connected" @click="sendEof">
            {{ t('terminal.ctrlD') }}
          </FpButton>
          <FpButton variant="ghost" :disabled="!connected" @click="sendInterrupt">
            {{ t('terminal.ctrlC') }}
          </FpButton>
          <FpButton
            v-permission="{ perm: 'node:execute', mode: 'view' }"
            variant="ghost"
            icon="oi oi-refresh"
            :loading="reconnecting"
            @click="reconnect"
          >
            {{ t('terminal.reconnect') }}
          </FpButton>
          <FpButton
            v-permission="{ perm: 'node:execute', mode: 'view' }"
            variant="danger"
            icon="oi oi-eraser"
            @click="handleClear"
          >
            {{ t('terminal.clear') }}
          </FpButton>
        </div>
      </div>
      <div ref="terminalContainer" class="terminal-container" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import '@xterm/xterm/css/xterm.css'
import FpButton from '@/components/ui/FpButton.vue'
import FpTag from '@/components/ui/FpTag.vue'
import { useWebSocket } from '@/composables/useWebSocket'

/**
 * Web 终端。传入 `nodeId` 时为节点远程终端（Agent Edge）：同一套 input / resize / output
 * 协议，端点改为 `/ws/terminal?node_id=`，另处理 `exit`（shell 退出）与 `error` 帧
 * （TERMINAL_DISABLED / TOO_MANY_TERMINALS 停止自动重连；EDGE_OFFLINE 交给退避重连等待 Agent 上线）。
 */
const props = defineProps<{ nodeId?: number; nodeName?: string }>()

const { t } = useI18n()

const terminalContainer = ref<HTMLElement>()
const connected = ref(false)
const reconnecting = ref(false)

let term: Terminal | null = null
let fitAddon: FitAddon | null = null

/** 会话被服务端明确结束（shell 退出 / 不可恢复错误）后停止自动重连，等用户点「重连」 */
let stopped = false

const wsTerm = useWebSocket(
  props.nodeId ? `/ws/terminal?node_id=${props.nodeId}` : '/ws/terminal',
  {
    onStatus: (connectedFlag) => {
      connected.value = connectedFlag
      // B4：reconnecting 由真实连接状态驱动（连接成功后清除 loading），替代固定 500ms 模拟
      if (connectedFlag) reconnecting.value = false
      if (connectedFlag) {
        term?.focus()
        sendResize()
      }
    },
    onMessage: (data) => {
      const msg = data as {
        type: string
        data?: string
        code?: number | string | null
        message?: string
      }
      if (msg.type === 'output' && msg.data && term) {
        term.write(msg.data)
      } else if (msg.type === 'exit') {
        term?.write(`\r\n${t('terminal.exited', { code: msg.code ?? '-' })}\r\n`)
        stopSession()
      } else if (msg.type === 'error') {
        const code = String(msg.code ?? '')
        const key = `terminal.err${code}`
        const text = ['TERMINAL_DISABLED', 'TOO_MANY_TERMINALS', 'EDGE_OFFLINE'].includes(code)
          ? t(key)
          : (msg.message ?? code)
        term?.write(`\r\n\x1b[33m[flamepanel] ${text}\x1b[0m\r\n`)
        if (code !== 'EDGE_OFFLINE') stopSession()
      }
    },
  },
)

function stopSession() {
  stopped = true
  connected.value = false
  wsTerm.close()
}

function sendResize() {
  if (!term) return
  wsTerm.send(JSON.stringify({ type: 'resize', cols: term.cols, rows: term.rows }))
}

function terminalTheme() {
  const v = (name: string, fallback: string) =>
    getComputedStyle(document.documentElement).getPropertyValue(name).trim() || fallback
  return {
    background: v('--fp-term-bg', '#1a1b1e'),
    foreground: v('--fp-term-fg', '#cdd6f4'),
    cursor: v('--fp-term-cursor', '#f5e0dc'),
    selectionBackground: v('--fp-term-selection', '#585b70'),
    black: v('--fp-term-black', '#45475a'),
    red: v('--fp-term-red', '#f38ba8'),
    green: v('--fp-term-green', '#a6e3a1'),
    yellow: v('--fp-term-yellow', '#f9e2af'),
    blue: v('--fp-term-blue', '#89b4fa'),
    magenta: v('--fp-term-magenta', '#f5c2e7'),
    cyan: v('--fp-term-cyan', '#94e2d5'),
    white: v('--fp-term-white', '#bac2de'),
    brightBlack: v('--fp-term-bright-black', '#585b70'),
    brightRed: v('--fp-term-bright-red', '#f38ba8'),
    brightGreen: v('--fp-term-bright-green', '#a6e3a1'),
    brightYellow: v('--fp-term-bright-yellow', '#f9e2af'),
    brightBlue: v('--fp-term-bright-blue', '#89b4fa'),
    brightMagenta: v('--fp-term-bright-magenta', '#f5c2e7'),
    brightCyan: v('--fp-term-bright-cyan', '#94e2d5'),
    brightWhite: v('--fp-term-bright-white', '#a6adc8'),
  }
}

function initTerminal() {
  if (!terminalContainer.value) return

  term = new Terminal({
    cursorBlink: true,
    cursorStyle: 'block',
    fontSize: 14,
    fontFamily: "'Cascadia Code', 'Fira Code', 'Consolas', monospace",
    theme: terminalTheme(),
    allowTransparency: true,
  })

  fitAddon = new FitAddon()
  term.loadAddon(fitAddon)
  term.open(terminalContainer.value)

  term.onData((data: string) => {
    // B4：连接断开时发送失败，本地回显提示，避免用户输入被静默丢弃
    const ok = wsTerm.send(JSON.stringify({ type: 'input', data }))
    if (!ok) {
      term?.write('\r\n[flamepanel] 连接已断开，输入未发送。点击「重连」恢复会话。\r\n')
    }
  })

  // 尺寸变化同步给 PTY（vim / top 等全屏程序依赖正确的行列数）
  term.onResize(() => sendResize())

  nextTick(() => fitAddon?.fit())
}

function connect() {
  wsTerm.connect()
}

function reconnect() {
  // B4：loading 由 onStatus 在连接成功时清除，这里只触发重连
  reconnecting.value = true
  if (stopped) {
    // 服务端已结束会话时连接已关闭：重新订阅即建立新连接（新 shell）
    stopped = false
    wsTerm.connect()
    return
  }
  wsTerm.reconnectNow()
}

function sendEof() {
  wsTerm.send(JSON.stringify({ type: 'input', data: '\x04' }))
}

function sendInterrupt() {
  wsTerm.send(JSON.stringify({ type: 'input', data: '\x03' }))
}

function handleClear() {
  term?.clear()
}

function handleResize() {
  nextTick(() => fitAddon?.fit())
}

onMounted(() => {
  initTerminal()
  connect()
  window.addEventListener('resize', handleResize)
})

onUnmounted(() => {
  window.removeEventListener('resize', handleResize)
  wsTerm.close()
  term?.dispose()
})
</script>

<style scoped>
.terminal-page {
  height: calc(100dvh - 116px);
  min-height: 360px;
  display: flex;
  flex-direction: column;
}
.terminal-card {
  margin: 0;
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: var(--fp-space-4);
  border-radius: var(--fp-radius-md);
  background: var(--fp-bg-elevated);
  border: 1px solid var(--fp-border);
}
.terminal-container {
  flex: 1;
  min-height: 0;
}
.header-right {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: var(--fp-space-2);
  align-items: center;
}
.card-header-title {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fp-space-2);
  justify-content: space-between;
  align-items: center;
  margin-bottom: var(--fp-space-3);
}
.card-header-title > span {
  font-size: 16px;
  font-weight: 600;
  white-space: nowrap;
}
@media (max-width: 640px) {
  .terminal-card {
    padding: var(--fp-space-2);
  }
}
</style>
