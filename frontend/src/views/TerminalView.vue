<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import '@xterm/xterm/css/xterm.css'
import { ElCard, ElMessage } from 'element-plus'

const terminalRef = ref<HTMLElement | null>(null)
let term: Terminal
let fitAddon: FitAddon
let ws: WebSocket | null = null

onMounted(() => {
  if (!terminalRef.value) return

  term = new Terminal({
    cursorBlink: true,
    fontSize: 14,
    fontFamily: 'Consolas, Monaco, "Courier New", monospace',
    theme: {
      background: '#1e1e1e',
      foreground: '#d4d4d4',
      cursor: '#ffffff',
    },
    rows: 30,
    cols: 120,
  })

  fitAddon = new FitAddon()
  term.loadAddon(fitAddon)
  term.open(terminalRef.value)

  fitAddon.fit()

  const token = localStorage.getItem('token') || ''
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
  ws = new WebSocket(
    `${protocol}//${window.location.host}/ws/terminal?cols=120&rows=30`
  )

  ws.onopen = () => {
    term.writeln('\x1b[32m✓ 终端已成功连接\x1b[0m')
    term.writeln('\x1b[33m欢迎使用 Flamepanel Web 终端\x1b[0m')
    term.writeln('')
  }

  ws.onmessage = (event) => {
    try {
      const data =
        typeof event.data === 'string'
          ? event.data
          : new TextDecoder().decode(event.data as ArrayBuffer)
      term.write(data)
    } catch (e) {
      console.error('终端数据解析失败', e)
    }
  }

  ws.onerror = () => {
    ElMessage.error('WebSocket 连接失败，后端是否已启动？')
  }

  ws.onclose = () => {
    term.writeln('\r\n\x1b[31m终端连接已断开\x1b[0m')
  }

  term.onData((data) => {
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(data)
    }
  })

  const resizeObserver = new ResizeObserver(() => {
    fitAddon.fit()
  })
  resizeObserver.observe(terminalRef.value)
})

onUnmounted(() => {
  ws?.close()
  term?.dispose()
})
</script>

<template>
  <div class="terminal-page">
    <ElCard class="terminal-card">
      <template #header>
        <div style="display: flex; justify-content: space-between; align-items: center">
          <span>Web 终端 (Linux Shell)</span>
          <span style="font-size: 12px; color: #909399">
            按 Ctrl+C 可中断当前命令 | 支持 bash/sh
          </span>
        </div>
      </template>
      <div ref="terminalRef" class="terminal-container"></div>
    </ElCard>
  </div>
</template>

<style scoped>
.terminal-card {
  height: calc(100vh - 60px);
}

.terminal-container {
  height: calc(100vh - 150px);
  background: #1e1e1e;
  padding: 8px;
  border-radius: 4px;
}
</style>
