export default {
  runtime: {
    title: 'Runtimes',
    default: 'Default',
    installed: 'Installed',
    notInstalled: 'Not installed',
    kindLabel: 'Language',
    installPath: 'Install path',
    setDefault: 'Set default',
    uninstall: 'Uninstall',
    installSuccess: 'Install task submitted',
    installFailed: 'Install failed',
    defaultSuccess: 'Default version updated',
    defaultFailed: 'Failed to set default',
    uninstallConfirm: 'Uninstall runtime v{version}?',
    uninstallSuccess: 'Uninstalled',
    uninstallFailed: 'Uninstall failed',
    resolvedVersion: 'Resolved version',
    node: {
      withPm2: 'Also install pm2',
      hint: "Major versions resolve to the latest patch release and are verified with SHA-256. Project start commands can use node / npm / pm2-runtime directly, or {'$'}RUNTIME for the runtime bin directory.",
    },
    python: {
      hint: 'Compiled from python.org sources (build dependencies installed automatically; set FLAMEPANEL_PYTHON_MIRROR for a mirror) and self-checked for ssl / sqlite3 and friends. Deploys create .venv and install requirements.txt; start commands may use gunicorn / uvicorn / python directly (resolved inside the venv), or be left empty to auto-detect Django / FastAPI / Flask.',
    },
    go: {
      hint: 'Official go.dev release, SHA-256 verified (FLAMEPANEL_GO_MIRROR / FLAMEPANEL_GOPROXY for mirrors). Projects with go.mod are built to .fp-bin/app on deploy; leave the start command empty to run it.',
    },
    java: {
      hint: 'Installs the Eclipse Temurin JDK (Adoptium, SHA-256 verified; FLAMEPANEL_JDK_MIRROR for a mirror). Repositories with mvnw / gradlew are built on deploy (tests skipped); an empty start command auto-detects java -jar, with JAVA_HOME and SERVER_PORT injected.',
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
