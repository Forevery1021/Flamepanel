# GitHub 源码迁移与二进制发布

[English](./en/github-migration.md)

## 本轮边界

- CNB 基线：`7288d9b199e1b09b529fbaae38303f4c82f38612`。
- GitHub 目标：`Forevery1021/Flamepanel`，现有公开仓库。
- 开发测试使用 CNB 云原生开发环境的独立目录，不覆盖 `/workspace`，不使用普通 build-start 运行门禁。
- 通过新分支和 PR 在现有 GitHub 历史之上导入 CNB 源码快照；CNB 提交历史保留在原仓库，不复制到 GitHub。不强推，不替换 GitHub main，不迁移 Issue/PR/秘密，不配置双向自动同步。
- GitHub 旧源码与 CNB 差异很大，保留历史不表示自动保留所有旧实现；合并前须审阅 PR 的实际内容差异。

## 双语文档状态

README、AGENTS 默认英文，分别保留 `.zh-CN.md` 中文版本。原有 `Doc/` 中文指南及生成 API 文档不删除。

`docs/en/` 提供 27 篇**精简英文译写指南**及状态索引。它们不是逐段完整翻译；详细表格、代码样例、历史验收、生成 API 说明、CHANGELOG、设计文档、辅助文档和技能文档仍需后续逐项翻译、复核。不得将本批次说成“全文英文化完成”。

## 自动构建与资产

Release 工作流配置 amd64/arm64 的 GNU、musl 四目标，GNU 通过 Zig 指定 glibc 2.28 基线。旧 ARMv7 条目暂不沿用；完整 feature 的 32 位依赖与运行时需要单独验证。不因配置 musl 就宣称实际产物已经验证为完全静态。

每个架构生成：

- `flamepanel-linux-<arch>.tar.gz`：`flamepanel`、`fpctl`、`fp-restore-probe`、`fp-restore-keycheck`。
- `flamepanel-agent-linux-<arch>.tar.gz`：`flamepanel-agent`。

前端独立打包一次，上传 `flamepanel-frontend.tar.gz`。最终严格检查九个非空常规归档，拒绝缺失、额外目录、符号链接等，再生成 `flamepanel-<version>-checksums.txt`。不会把未打包前端目录或 `export-openapi` 混入发布资产。

安装器已有归档命名保持兼容；其解包后只选择预期面板/CLI 可执行文件，新增恢复工具不会自动成为服务，也不扩大秘密读取授权。

## 版本与正式发布

版本必须为不含 v 前缀的有效 SemVer，且与内核 Cargo.toml 一致。tag 取 `v*`，手动输入通过环境变量传递，不直接拼入 shell；手动不再把分支名误当版本，默认读取内核 package.version。手动构建默认仅上传经校验的 Actions 产物；显式设置 `create_draft=true` 才生成**草稿 Release**。

普通 job 只读，最终发布 job 才有 contents:write。本轮推分支、提 PR 不等于创建 tag、合并或发布正式版本。签名、SBOM、来源证明、实际四架构构建与真机验收须分别记录，配置存在不代表执行成功。

## 测试与后续

在 CNB 最终工作树运行新增离线资产回归及 `scripts/ci-local.sh --keep-going all`。记录工作树摘要、工具链、逐项结果和完整日志。工作流 lint、本机 release/打包冒烟、跨架构构建、实际安装验收是不同层次，不相互冒充。

注释清理保持运行时代码、协议键、用户可见文案、语言包和安全指令不变；本批次只做已审查入口文件，其余中文注释候选清单需要继续按模块处理。

令牌不进入仓库、Git URL、日志或 PR；Actions 使用临时 GITHUB_TOKEN。建议任务结束后撤销此次临时令牌。
