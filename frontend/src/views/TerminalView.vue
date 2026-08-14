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
          <FpButton v-permission="{ perm: 'node:execute', mode: 'view' }" variant="ghost" icon="oi oi-refresh" :loading="reconnecting" @click="reconnect">
            {{ t('terminal.reconnect') }}
          </FpButton>
          <FpButton v-permission="{ perm: 'node:execute', mode: 'view' }" variant="danger" icon="oi oi-eraser" @click="handleClear">
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

const { t } = useI18n()

const terminalContainer = ref<HTMLElement>()
const connected = ref(false)
const reconnecting = ref(false)

let term: Terminal | null = null
let fitAddon: FitAddon | null = null

const wsTerm = useWebSocket('/ws/terminal', {
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
    wsTerm.send(JSON.stringify({ type: 'input', data }))
  })

  nextTick(() => fitAddon?.fit())
}

function connect() {
  wsTerm.connect()
}

function reconnect() {
  reconnecting.value = true
  wsTerm.reconnectNow()
  setTimeout(() => {
    reconnecting.value = false
  }, 500)
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
