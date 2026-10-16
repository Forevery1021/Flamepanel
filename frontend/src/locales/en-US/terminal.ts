export default {
  terminal: {
    title: 'Web Terminal',
    connected: 'Connected',
    disconnected: 'Disconnected',
    reconnect: 'Reconnect',
    clear: 'Clear',
    ctrlC: 'Ctrl+C',
    ctrlD: 'Ctrl+D',
    remoteTitle: 'Remote Terminal · {name}',
    exited: '[flamepanel] Session ended (exit code {code}). Click "Reconnect" to start a new one.',
    errTERMINAL_DISABLED:
      'Remote terminal is disabled on this node (set AGENT_TERMINAL=1 and restart the Agent).',
    errTOO_MANY_TERMINALS: 'Too many terminals are open on this node. Close another one first.',
    errEDGE_OFFLINE:
      'The node Agent is not online via Edge (AGENT_MODE=edge or both). Retrying automatically.',
  },
}
