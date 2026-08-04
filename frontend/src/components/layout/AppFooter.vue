<template>
  <footer class="app-footer">
    <span>FlamePanel v{{ version }}</span>
    <span class="app-footer__sep">·</span>
    <span class="app-footer__status">
      <span class="dot" :class="online ? 'green' : 'red'" />
      {{ online ? t('topbar.panelOnline') : t('topbar.panelOffline') }}
    </span>
  </footer>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
const version = ref('0.1.0')
const online = ref(true)
let timer: number | null = null

async function ping() {
  try {
    const res = await fetch('/api/health', { signal: AbortSignal.timeout(3000) })
    online.value = res.ok
  } catch {
    online.value = false
  }
}

onMounted(() => {
  ping()
  timer = window.setInterval(ping, 30000)
})
onUnmounted(() => {
  if (timer !== null) clearInterval(timer)
})
</script>

<style scoped>
.app-footer {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
  height: 34px;
  padding: 0 var(--fp-space-4);
  border-top: 1px solid var(--fp-border);
  font-size: 12px;
  color: var(--fp-text-muted);
  flex-shrink: 0;
}
.app-footer__sep {
  opacity: 0.5;
}
.app-footer__status {
  display: flex;
  align-items: center;
  gap: 6px;
}
.dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
}
.dot.green {
  background: var(--fp-success);
}
.dot.red {
  background: var(--fp-danger);
}
</style>
