export default {
  terminal: {
    title: 'Web 终端',
    connected: '已连接',
    disconnected: '未连接',
    reconnect: '重连',
    clear: '清屏',
    ctrlC: 'Ctrl+C',
    ctrlD: 'Ctrl+D',
    remoteTitle: '远程终端 · {name}',
    exited: '[flamepanel] 会话已结束（退出码 {code}），点击「重连」开启新会话。',
    errTERMINAL_DISABLED:
      '该节点 Agent 未开启远程终端（在节点上设置 AGENT_TERMINAL=1 后重启 Agent）。',
    errTOO_MANY_TERMINALS: '该节点打开的终端已达上限，请先关闭其他终端。',
    errEDGE_OFFLINE: '节点 Agent 未通过 Edge 通道在线（AGENT_MODE=edge 或 both），稍后自动重试。',
  },
}
