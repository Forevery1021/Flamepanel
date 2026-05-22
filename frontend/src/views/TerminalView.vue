<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import 'xterm/css/xterm.css'

const terminalRef = ref<HTMLElement>()
let term: Terminal
let fitAddon: FitAddon
let ws: WebSocket | null = null

onMounted(() => {
  term = new Terminal({
    cursorBlink: true,
    fontSize: 14,
    theme: {
      background: '#1e1e1e',
      foreground: '#d4d4d4'
    }
  })

  fitAddon = new FitAddon()
  term.loadAddon(fitAddon)

  if (terminalRef.value) {
    term.open(terminalRef.value)
    fitAddon.fit()
  }

  // 连接 WebSocket
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
  ws = new WebSocket(`${protocol}//${window.location.host}/ws/terminal?cols=120&rows=30`)

  ws.onopen = () => {
    term.writeln('\x1b[32m终端已连接...\x1b[0m')
  }

  ws.onmessage = (event) => {
    term.write(typeof event.data === 'string' ? event.data : new TextDecoder().decode(event.data))
  }

  term.onData((data) => {
    ws?.send(data)
  })

  window.addEventListener('resize', () => fitAddon.fit())
})

onUnmounted(() => {
  ws?.close()
  term.dispose()
})
</script>

<template>
  <ElCard class="terminal-card h-full">
    <template #header>Web 终端 (Linux Shell)</template>
    <div ref="terminalRef" class="terminal-container"></div>
  </ElCard>
</template>

<style scoped>
.terminal-container {
  height: calc(100vh - 180px);
  background: #1e1e1e;
}
</style>