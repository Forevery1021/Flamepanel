import { ref } from 'vue'

export function useTerminal() {
  const sessionId = ref('')
  const connected = ref(false)

  return {
    sessionId,
    connected,
  }
}
