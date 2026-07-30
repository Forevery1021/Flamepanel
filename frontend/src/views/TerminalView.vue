<template>
  <div class="view-container" style="height: calc(100vh - 116px); display: flex; flex-direction: column;">
    <el-card shadow="hover" class="terminal-card" style="flex: 1; display: flex; flex-direction: column;">
      <template #header>
        <div class="card-header-title">
          <span>{{ t('terminal.title') }}</span>
          <div class="header-right">
            <el-tag :type="connected ? 'success' : 'danger'" size="small" effect="dark">
              {{ connected ? t('terminal.connected') : t('terminal.disconnected') }}
            </el-tag>
            <el-button size="small" :disabled="!connected" @click="sendEof">{{ t('terminal.ctrlD') }}</el-button>
            <el-button size="small" :disabled="!connected" @click="sendInterrupt">{{ t('terminal.ctrlC') }}</el-button>
            <el-button size="small" @click="reconnect" :loading="reconnecting">{{ t('terminal.reconnect') }}</el-button>
            <el-button size="small" type="danger" @click="handleClear">{{ t('terminal.clear') }}</el-button>
          </div>
        </div>
      </template>
      <div ref="terminalContainer" class="terminal-container" />
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import '@xterm/xterm/css/xterm.css'

const { t } = useI18n()

const terminalContainer = ref<HTMLElement>()
const connected = ref(false)
const reconnecting = ref(false)

let term: Terminal | null = null
let fitAddon: FitAddon | null = null
let ws: WebSocket | null = null
let reconnectTimer: ReturnType<typeof setTimeout> | null = null

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
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({ type: 'input', data }))
    }
  })

  nextTick(() => fitAddon?.fit())
}

function connect() {
  const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:'
  ws = new WebSocket(`${protocol}//${location.host}/ws/terminal`)

  ws.onopen = () => {
    connected.value = true
    term?.focus()
  }

  ws.onclose = () => {
    connected.value = false
    if (reconnectTimer) clearTimeout(reconnectTimer)
    reconnectTimer = setTimeout(() => {
      if (!connected.value) connect()
    }, 3000)
  }

  ws.onerror = () => {
    connected.value = false
  }

  ws.onmessage = (ev: MessageEvent) => {
    try {
      const msg = JSON.parse(ev.data)
      if (msg.type === 'output' && msg.data && term) {
        term.write(msg.data)
      }
    } catch {
      // ignore
    }
  }
}

function reconnect() {
  reconnecting.value = true
  ws?.close()
  if (reconnectTimer) clearTimeout(reconnectTimer)
  setTimeout(() => {
    connect()
    reconnecting.value = false
  }, 500)
}

function sendEof() {
  if (ws && ws.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify({ type: 'input', data: '\x04' }))
  }
}

function sendInterrupt() {
  if (ws && ws.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify({ type: 'input', data: '\x03' }))
  }
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
  ws?.close()
  if (reconnectTimer) clearTimeout(reconnectTimer)
  term?.dispose()
})
</script>

<style scoped>
.terminal-card { margin: 0; }
.terminal-container { flex: 1; min-height: 0; }
.header-right { display: flex; gap: 8px; align-items: center; }
.card-header-title { display: flex; justify-content: space-between; align-items: center; }
.card-header-title span { font-size: 16px; font-weight: 600; }
</style>
