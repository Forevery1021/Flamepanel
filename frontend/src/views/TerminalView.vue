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

  // 初始化终端
  term = new Terminal({
    cursorBlink: true,
    fontSize: 14,
    fontFamily: 'Consolas, Monaco, monospace',
    theme: {
      background: '#1e1e1e',
      foreground: '#d4d4d4',
      cursor: '#ffffff'
    },
    rows: 30,
    cols: 120,
  })

  fitAddon = new FitAddon()
  term.loadAddon(fitAddon)

  // 打开终端
  term.open(terminalRef.value)
  fitAddon.fit()

  // 连接 WebSocket
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
  ws = new WebSocket(`${protocol}//${window.location.host}/ws/terminal?cols=120&rows=30`)

  ws.onopen = () => {
    term.writeln('\x1b[32m✓ 终端已成功连接\x1b[0m')
    term.writeln('\x1b[33m欢迎使用 Ops Panel Web 终端\x1b[0m\r\n')
  }

  ws.onmessage = (event) => {
    try {
      const data = typeof event.data === 'string' 
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

  // 前端输入发送到后端
  term.onData((data) => {
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(data)
    }
  })

  // 窗口大小变化时自适应
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
  <ElCard class="terminal-card" style="height: 100%;">
    <template #header>
      <div class="flex justify-between items-center">
        <span>Web 终端 (Linux Shell)</span>
        <span class="text-xs text-gray-500">按 Ctrl+C 可中断当前命令</span>
      </div>
    </template>
    
    <div ref="terminalRef" class="terminal-container"></div>
  </ElCard>
</template>

<style scoped>
.terminal-card {
  height: calc(100vh - 40px);
}

.terminal-container {
  height: calc(100vh - 120px);
  background: #1e1e1e;
  padding: 8px;
  border-radius: 4px;
}
</style>