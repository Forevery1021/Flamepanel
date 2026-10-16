export default {
  runtime: {
    title: '运行环境',
    default: '默认',
    installed: '已安装',
    notInstalled: '未安装',
    kindLabel: '语言',
    installPath: '安装路径',
    setDefault: '设为默认',
    uninstall: '卸载',
    installSuccess: '安装任务已提交',
    installFailed: '安装失败',
    defaultSuccess: '默认版本已更新',
    defaultFailed: '设置默认失败',
    uninstallConfirm: '确定要卸载 v{version} 运行时吗？',
    uninstallSuccess: '卸载成功',
    uninstallFailed: '卸载失败',
    resolvedVersion: '实际版本',
    node: {
      withPm2: '同时安装 pm2',
      hint: "主版本号会解析为最新补丁版并校验 SHA-256。项目启动命令可直接写 node / npm / pm2-runtime，或用 {'$'}RUNTIME 引用运行时 bin 目录。",
    },
    python: {
      hint: '从 python.org 源码编译（自动安装编译依赖，可用 FLAMEPANEL_PYTHON_MIRROR 指定镜像），装好后自检 ssl / sqlite3 等模块。部署时自动创建 .venv 并安装 requirements.txt；启动命令可直接写 gunicorn / uvicorn / python（按 venv 补全），留空时按 Django / FastAPI / Flask 约定探测。',
    },
    go: {
      hint: '从 go.dev 下载官方发行包并校验 SHA-256（可用 FLAMEPANEL_GO_MIRROR / FLAMEPANEL_GOPROXY 指定镜像）。含 go.mod 的项目部署时自动 go build 为 .fp-bin/app，启动命令留空即运行该产物。',
    },
    java: {
      hint: '安装 Eclipse Temurin JDK（Adoptium，校验 SHA-256，可用 FLAMEPANEL_JDK_MIRROR 指定镜像）。仓库带 mvnw / gradlew 时部署自动构建（跳过测试），启动命令留空时自动探测 java -jar，并注入 JAVA_HOME 与 SERVER_PORT。',
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
