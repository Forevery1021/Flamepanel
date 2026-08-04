<template>
  <div class="view-container terminal-page">
    <div class="panel terminal-card">
      <div class="card-header-title">
        <span>{{ t('terminal.title') }}</span>
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
          <FpButton variant="ghost" icon="oi oi-refresh" :loading="reconnecting" @click="reconnect">
            {{ t('terminal.reconnect') }}
          </FpButton>
          <FpButton variant="danger" icon="oi oi-eraser" @click="handleClear">
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
import { connectWithRetry } from '@/utils/ws'
import { useI18n } from 'vue-i18n'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import '@xterm/xterm/css/xterm.css'
import FpButton from '@/components/ui/FpButton.vue'
import FpTag from '@/components/ui/FpTag.vue'

const { t } = useI18n()

const terminalContainer = ref<HTMLElement>()
const connected = ref(false)
const reconnecting = ref(false)

let term: Terminal | null = null
let fitAddon: FitAddon | null = null
let wsConn: ReturnType<typeof connectWithRetry> | null = null

function initTerminal() {
  if (!terminalContainer.value) return

  term = new Terminal({
    cursorBlink: true,
    cursorStyle: 'block',
    fontSize: 14,
    fontFamily: "'Cascadia Code', 'Fira Code', 'Consolas', monospace",
    theme: {
      background: '#1a1b1e',
      foreground: '#cdd6f4',
      cursor: '#f5e0dc',
      selectionBackground: '#585b70',
      black: '#45475a',
      red: '#f38ba8',
      green: '#a6e3a1',
      yellow: '#f9e2af',
      blue: '#89b4fa',
      magenta: '#f5c2e7',
      cyan: '#94e2d5',
      white: '#bac2de',
      brightBlack: '#585b70',
      brightRed: '#f38ba8',
      brightGreen: '#a6e3a1',
      brightYellow: '#f9e2af',
      brightBlue: '#89b4fa',
      brightMagenta: '#f5c2e7',
      brightCyan: '#94e2d5',
      brightWhite: '#a6adc8',
    },
    allowTransparency: true,
  })

  fitAddon = new FitAddon()
  term.loadAddon(fitAddon)
  term.open(terminalContainer.value)

  term.onData((data: string) => {
    wsConn?.send(JSON.stringify({ type: 'input', data }))
  })

  nextTick(() => fitAddon?.fit())
}

function connect() {
  wsConn = connectWithRetry('/ws/terminal', {
    onStatus: (connectedFlag) => {
      connected.value = connectedFlag
      if (connectedFlag) term?.focus()
    },
    onMessage: (data) => {
      const msg = data as { type: string; data: string }
      if (msg.type === 'output' && msg.data && term) {
        term.write(msg.data)
      }
    },
  })
}

function reconnect() {
  reconnecting.value = true
  wsConn?.reconnectNow()
  setTimeout(() => {
    reconnecting.value = false
  }, 500)
}

function sendEof() {
  wsConn?.send(JSON.stringify({ type: 'input', data: '\x04' }))
}

function sendInterrupt() {
  wsConn?.send(JSON.stringify({ type: 'input', data: '\x03' }))
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
  wsConn?.close()
  term?.dispose()
})
</script>

<style scoped>
.terminal-page {
  height: calc(100vh - 116px);
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
  gap: var(--fp-space-2);
  align-items: center;
}
.card-header-title {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: var(--fp-space-3);
}
.card-header-title span {
  font-size: 16px;
  font-weight: 600;
}
</style>
