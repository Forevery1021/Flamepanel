export default {
  runtime: {
    title: 'ランタイム',
    default: 'デフォルト',
    installed: 'インストール済み',
    notInstalled: '未インストール',
    kindLabel: '言語',
    installPath: 'インストール先',
    setDefault: 'デフォルトに設定',
    uninstall: 'アンインストール',
    installSuccess: 'インストールタスクを送信しました',
    installFailed: 'インストールに失敗しました',
    defaultSuccess: 'デフォルトバージョンを更新しました',
    defaultFailed: 'デフォルト設定に失敗しました',
    uninstallConfirm: 'ランタイム v{version} をアンインストールしますか？',
    uninstallSuccess: 'アンインストールしました',
    uninstallFailed: 'アンインストールに失敗しました',
    resolvedVersion: '実際のバージョン',
    node: {
      withPm2: 'pm2 も同時にインストール',
      hint: "メジャーバージョンは最新のパッチ版に解決され、SHA-256 で検証されます。プロジェクトの起動コマンドには node / npm / pm2-runtime をそのまま書くか、{'$'}RUNTIME でランタイムの bin ディレクトリを参照できます。",
    },
    python: {
      hint: 'python.org のソースからビルドします（ビルド依存は自動インストール、FLAMEPANEL_PYTHON_MIRROR でミラー指定可）。インストール後に ssl / sqlite3 などを自己診断します。デプロイ時に .venv を作成して requirements.txt をインストールし、起動コマンドは gunicorn / uvicorn / python をそのまま書けます（venv 内で解決）。空欄なら Django / FastAPI / Flask の規約で自動検出します。',
    },
    go: {
      hint: 'go.dev の公式リリースを SHA-256 検証付きで取得します（FLAMEPANEL_GO_MIRROR / FLAMEPANEL_GOPROXY でミラー指定可）。go.mod のあるプロジェクトはデプロイ時に .fp-bin/app へビルドされ、起動コマンドが空ならそれを実行します。',
    },
    java: {
      hint: 'Eclipse Temurin JDK（Adoptium、SHA-256 検証、FLAMEPANEL_JDK_MIRROR でミラー指定可）をインストールします。mvnw / gradlew を含むリポジトリはデプロイ時に自動ビルド（テスト省略）され、起動コマンドが空なら java -jar を自動検出し、JAVA_HOME と SERVER_PORT を注入します。',
    },
    kinds: {
      php: 'PHP',
      java: 'Java',
      node: 'Node.js',
      python: 'Python',
      go: 'Go',
      tomcat: 'Tomcat',
    },
  },
}
