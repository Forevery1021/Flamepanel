# CNB 开发镜像：Rust Remote-SSH 环境

## 问题与修复

Rust 工具链由 `.cnb/npc/Dockerfile` 安装在 `/usr/local/rustup` 和
`/usr/local/cargo`。CNB Remote-SSH 服务使用 `UsePAM=no`，SSH 会话未必继承
Docker `ENV`。即使命令路径正确，缺失这两个 home 仍会使 rustup 报：

```text
rustup could not choose a version of cargo to run
```

开发镜像将 `10-flamepanel-rust.conf` 安装到 OpenSSH 的配置目录，通过服务端
`SetEnv` 为登录及非交互会话设置这两个固定路径。不依赖客户端 `SendEnv`、PAM、
`/etc/environment` 或用户 shell 启动文件，不重装工具链，也不修改 SSH 认证策略。
路径必须与 NPC 镜像安装位置保持一致。

## 镜像构建

在**仓库根目录**提供构建上下文：

```bash
docker build -f .ide/Dockerfile -t flamepanel-dev:rust-ssh .
```

Dockerfile 会确认基础镜像的 sshd 配置包含 `/etc/ssh/sshd_config.d/*.conf`。
若基础镜像改变该约定，构建应失败，不静默产出未生效的配置。

CNB 的 `.cnb.yml` 使用 `.ide/Dockerfile`。修改镜像定义不会自动修复已经运行的
云环境；需要在变更进入目标分支后，按 CNB 的镜像构建/环境生命周期方式启用新镜像。
不要为了应用修复直接关闭或删除含有未提交工作的环境。

## CNB 构建输入与前端工具链

CNB 不会自动传入完整仓库作为 Docker 构建上下文。`.cnb.yml` 的 `docker.build.by`
必须包含所有仓库 `COPY` 输入：Rust SSH 配置、`.node-version` 和 `frontend/package.json`。
遗漏时即使本地 `docker build --check` 通过，云环境 Prepare 仍会因 COPY 文件不存在失败。
不能仅设置 `versionBy` 而省略 `by`；新增 COPY 文件时必须同步列表。

Node 精确版本由 `.node-version` 声明，npm 由 `frontend/package.json` 的 `packageManager`
声明。官方 Node 镜像标签也固定相同版本，由前端工具链测试检查，升级时需要同步更新
两个 Dockerfile 的标签。开发镜像显式安装这套 Node/npm，不依赖上游 NPC 镜像自带版本
或 PATH 意外选中 NodeSource 的另一版本。

前端预热改用 `npm ci`，安装过程、退出码和超时可在 CNB 阶段日志查看。
`cargo fetch` 仍保留原有后台策略，未包含在这次前端修复范围内。

## 隔离回归检查

在已经含有 Rust 与 OpenSSH 的 CNB 开发容器中，以 root 执行：

```bash
# 验证仓库中的候选配置（不改动或重载当前 SSH 服务）
sh .ide/test-rust-ssh.sh

# 新镜像内再验证实际安装的配置文件
sh .ide/test-rust-ssh.sh /etc/ssh/sshd_config.d/10-flamepanel-rust.conf
```

检查脚本启动只监听 `127.0.0.1:39222` 的独立 sshd，使用临时客户端密钥与主机密钥，
严格校验由本次生成的主机公钥，不使用 `StrictHostKeyChecking=no`。
若端口被占用，选另一个高位端口：

```bash
FLAMEPANEL_TEST_SSH_PORT=39223 sh .ide/test-rust-ssh.sh
```

验证内容：

- 服务端和客户端均移除继承的 `RUSTUP_HOME`、`CARGO_HOME`；
- `UsePAM=no` 下由配置注入正确路径；
- 非交互 SSH 命令可以调用 rustc、Cargo、rustfmt、Clippy 和 rustup；
- 在临时目录离线构建、测试并运行一个无第三方依赖的 Rust 冒烟项目；
- 脚本退出时清理测试 sshd、临时密钥和测试文件。

检查只使用容器已有的 OpenSSH、coreutils 与 Rust 工具链，不下载依赖。
测试 sshd 为临时 authorized_keys 设置的 `StrictModes no` 只存在于测试临时配置，
不写入正式镜像的 SSH 配置；该 sshd 禁止密码认证、终端分配和端口转发。

负例检查：不加载修复配置时必须失败：

```bash
if sh .ide/test-rust-ssh.sh /dev/null; then
  echo '错误：缺少 Rust SetEnv 的配置不应通过' >&2
  exit 1
fi
```

这些是工具链/连接层检查，不替代 FlamePanel 的完整业务测试与 CI。
仓库候选配置检查通过，也不等于已验证完整镜像构建或 CNB 新环境启动。

## 已运行环境的临时办法

暂不能重建环境时，可在每次获准的远程命令会话中显式设置：

```bash
export RUSTUP_HOME=/usr/local/rustup
export CARGO_HOME=/usr/local/cargo
export PATH=/usr/local/cargo/bin:$PATH
```

这是临时命令环境，不会跨 SSH 调用自动保留。不要通过重新下载 Rust 来掩盖目录配置问题。
