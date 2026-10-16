export default {
  terminal: {
    title: 'Web ターミナル',
    connected: '接続済み',
    disconnected: '未接続',
    reconnect: '再接続',
    clear: 'クリア',
    ctrlC: 'Ctrl+C',
    ctrlD: 'Ctrl+D',
    remoteTitle: 'リモートターミナル · {name}',
    exited:
      '[flamepanel] セッションが終了しました（終了コード {code}）。「再接続」で新しいセッションを開始します。',
    errTERMINAL_DISABLED:
      'このノードの Agent はリモートターミナルが無効です（AGENT_TERMINAL=1 を設定して Agent を再起動）。',
    errTOO_MANY_TERMINALS:
      'このノードで開いているターミナルが上限に達しました。他のターミナルを閉じてください。',
    errEDGE_OFFLINE:
      'ノードの Agent が Edge 経由でオンラインではありません（AGENT_MODE=edge または both）。自動で再試行します。',
  },
}
