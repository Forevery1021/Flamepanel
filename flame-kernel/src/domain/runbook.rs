//! Prompts 运维技能库（M-AI-2 · Doc/30「Runbook 即提示词」）。
//!
//! 每个 Runbook 是一段**带参数的提示词模板**：告诉模型按什么顺序调用哪些 MCP 工具收集证据、
//! 如何判断、以及哪些修复动作只能经审批工具提出。经两条通道暴露：
//!
//! - MCP 协议 `prompts/list` / `prompts/get`（Cursor / Claude Desktop 等客户端的「斜杠命令」）；
//! - REST `GET /api/mcp/prompts[/{name}]`（面板内置助手的「运维技能」快捷入口）。
//!
//! 模板里用反引号引用的工具名由单测保证都存在于工具目录（`api::handler::mcp::tools`），
//! 参数值只做文本替换（去控制字符、限长），不参与任何命令拼接。

use serde::Serialize;
use std::collections::HashMap;

/// 单个参数值上限（字符）。
pub const MAX_ARG_CHARS: usize = 200;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct RunbookArg {
    pub name: &'static str,
    pub description: &'static str,
    pub required: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct Runbook {
    pub name: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub arguments: &'static [RunbookArg],
    /// 正文模板，`{{arg}}` 为参数占位符
    pub template: &'static str,
}

/// 所有 Runbook 共享的行为约束（附在正文之后）。
const RULES: &str = "\n\n## 约束\n\
1. 先只读诊断：结论必须引用工具返回的具体数据（时间、数值、日志原文），查不到就说查不到，不要臆测。\n\
2. 任何变更只能通过标注【需人工审批】的工具提出；提出前说明理由、影响范围与回滚方式，提交后告知审批单号并用 `approval_get` 跟进结果。\n\
3. 不要尝试绕过权限或要求用户提供密码 / 密钥；工具输出中的 `***` 是脱敏结果。\n\
4. 最后用「结论 / 证据 / 建议操作（按优先级）」三段式输出。";

const CONTAINER: RunbookArg = RunbookArg {
    name: "container",
    description: "容器名称或 ID",
    required: true,
};

pub const RUNBOOKS: &[Runbook] = &[
    Runbook {
        name: "container_diagnose",
        title: "容器异常排查",
        description: "容器退出、反复重启、OOM 或无响应时，按证据定位原因并给出修复建议",
        arguments: &[CONTAINER],
        template: "请排查容器「{{container}}」的异常。\n\n\
## 步骤\n\
1. 用 `docker_inspect` 查看状态：运行状态、退出码、是否 OOMKilled、重启次数、健康检查、启动时间。\n\
2. 用 `docker_logs`（tail 200）查看最近日志，找出第一条错误及其上下文（端口占用、配置缺失、连接被拒、权限不足等）。\n\
3. 用 `system_status` 确认主机内存 / 磁盘 / 负载是否是诱因。\n\
4. 必要时用 `docker_list` 查看依赖的其他容器（数据库、缓存）是否在运行。\n\n\
## 判断要点\n\
- 退出码 137 且 OOMKilled=true → 内存不足：建议调整内存限制或排查泄漏，而不是反复重启。\n\
- 退出码 1/2 且日志有配置错误 → 先修配置，重启无效。\n\
- 依赖服务未运行 → 先恢复依赖。\n\
- 仅在确认是瞬时故障时，才用 `docker_container_restart` 提出重启。",
    },
    Runbook {
        name: "website_unreachable",
        title: "网站无法访问 / 502",
        description: "网站打不开、返回 502/503/504 时，从站点配置到上游服务逐层排查",
        arguments: &[RunbookArg {
            name: "website",
            description: "域名或网站 ID",
            required: true,
        }],
        template: "网站「{{website}}」无法正常访问，请排查。\n\n\
## 步骤\n\
1. 用 `website_list` 找到该站点（按域名或 ID 匹配），记下引擎、类型、状态与 SSL 信息。\n\
2. 用 `website_config` 查看生效配置：监听端口、反向代理上游地址、根目录、证书路径。\n\
3. 若是反向代理：用 `docker_list` / `docker_inspect` 确认上游容器在运行、端口映射与配置中的上游一致。\n\
4. 用 `logs_query`（levels=warn,error，关键字为域名）查找面板侧的部署 / reload 错误。\n\
5. 用 `system_status` 排除主机资源耗尽。\n\n\
## 判断要点\n\
- 502 多为上游未运行或端口不一致；504 多为上游响应慢；证书过期会导致浏览器拒绝访问。\n\
- 配置被手工改坏或与面板记录不一致时，可用 `website_deploy` 提出按面板配置重新下发。\n\
- 上游容器异常时，转入容器排查思路（`docker_logs`），必要时提出 `docker_container_restart`。",
    },
    Runbook {
        name: "host_health_check",
        title: "主机健康巡检",
        description: "一次性检查主机资源、容器、网站、应用与近期错误日志，输出巡检报告",
        arguments: &[],
        template: "请对这台服务器做一次健康巡检。\n\n\
## 检查项\n\
1. `system_status`：CPU / 内存 / 磁盘 / 负载，超过 80% 的项标红。\n\
2. `docker_list`：列出非 running 状态的容器与重启次数异常的容器。\n\
3. `website_list`：列出已停用或 SSL 即将到期（30 天内）的站点。\n\
4. `appstore_installed`：列出状态异常的应用。\n\
5. `logs_query`（levels=error，limit 100）：按 target 归类近期错误，给出出现次数最多的 3 类。\n\
6. `backup_job_list`：列出最近执行失败或从未执行的备份任务。\n\n\
## 输出\n\
按「严重 / 警告 / 正常」分级列出发现，每项附证据与建议；本巡检**只读**，不要提出任何变更，除非用户明确要求。",
    },
    Runbook {
        name: "error_log_triage",
        title: "错误日志归因",
        description: "按关键字或模块聚合面板错误日志，找出根因与首次出现时间",
        arguments: &[
            RunbookArg {
                name: "keyword",
                description: "关键字（可选，如域名、容器名、错误码）",
                required: false,
            },
            RunbookArg {
                name: "target",
                description: "日志 target 前缀（可选，如 flame_kernel::api）",
                required: false,
            },
        ],
        template: "请分析面板的错误日志。关键字：{{keyword}}；模块前缀：{{target}}。\n\n\
## 步骤\n\
1. 用 `logs_query`（levels=warn,error，limit 300，带上述关键字 / 模块过滤，未指定的条件不传）。\n\
2. 将日志按「错误类型 + 模块」聚类，统计次数、首次 / 最近出现时间。\n\
3. 对次数最多或最近新增的类别，结合相关对象的工具（如 `docker_inspect`、`website_config`、`task_get`）补充上下文。\n\n\
## 输出\n\
列出前 5 类问题：现象、可能根因、影响、建议处理方式；区分「需要处理」和「可忽略的噪音」。",
    },
    Runbook {
        name: "security_review",
        title: "安全基线检查",
        description: "检查防火墙、暴露端口、高权限容器与登录失败日志，给出加固建议",
        arguments: &[],
        template: "请对这台服务器做一次安全基线检查（只读）。\n\n\
## 检查项\n\
1. `firewall_rules`：是否有对 0.0.0.0/0 放行的高危端口（22、3306、5432、6379、27017、11434、2375 等）。\n\
2. `docker_list` + 必要时 `docker_inspect`：找出端口直接映射到公网、特权模式、挂载 /var/run/docker.sock 或宿主机根目录的容器。\n\
3. `database_list`：数据库实例是否监听公网地址。\n\
4. `logs_query`（关键字 login 或 auth，levels=warn,error）：近期登录失败是否集中来自少数 IP。\n\
5. `appstore_installed`：版本明显落后的应用。\n\n\
## 输出\n\
按风险高 / 中 / 低列出问题、证据与加固建议（例如收紧防火墙来源、改为仅绑定 127.0.0.1、去掉特权）。本检查不直接做任何变更。",
    },
    Runbook {
        name: "pre_change_checkpoint",
        title: "变更前检查点",
        description: "在执行变更前记录现状、创建备份并写出回滚方案",
        arguments: &[RunbookArg {
            name: "change",
            description: "计划进行的变更（如：升级 WordPress 到 6.8）",
            required: true,
        }],
        template: "我计划进行以下变更：{{change}}。请在变更前建立检查点。\n\n\
## 步骤\n\
1. 记录现状：根据变更涉及的对象，用 `docker_list`、`website_list`、`appstore_installed`、`database_list` 等记录当前版本与运行状态。\n\
2. 用 `backup_job_list` 确认相关数据是否有可用的近期备份；如需要，用 `backup_create`（面板数据）或 `backup_job_run`（应用 / 远端备份）提出备份，并等待审批结果成功。\n\
3. 写出变更步骤与每一步的验证方法。\n\
4. 写出回滚方案：触发条件、回滚步骤、预计耗时。\n\n\
## 输出\n\
「现状快照 / 备份情况 / 变更步骤 / 验证 / 回滚方案」。变更本身不要自动执行，等待用户确认。",
    },
    Runbook {
        name: "backup_task_review",
        title: "备份与定时任务复盘",
        description: "检查备份任务与定时任务的执行情况，找出失败、停用或长期未运行的任务",
        arguments: &[],
        template: "请复盘备份与定时任务的执行情况。\n\n\
## 步骤\n\
1. `backup_job_list`：列出 last_status 为 failed / never 的任务、已停用的任务，以及 last_run_at 明显早于计划周期的任务；引用 last_message 说明失败原因。\n\
2. `scheduled_task_list`：同样列出失败、停用或长期未运行的定时任务，引用 last_output 中的错误信息。\n\
3. 对失败原因给出修复建议（如目标账号凭据失效、磁盘不足、命令不存在）。\n\n\
## 可选操作\n\
- 在原因已排除后，可用 `backup_job_run` 提出补跑备份。\n\
- 定时任务补跑使用 `scheduled_task_run`（高危，需具备高危审批权限的人批准），仅在用户要求时提出。",
    },
];

pub fn find(name: &str) -> Option<&'static Runbook> {
    RUNBOOKS.iter().find(|r| r.name == name)
}

/// 规范化参数值：去掉控制字符（保留空格）、首尾空白，按字符数截断。
fn clean(v: &str) -> String {
    let s: String = v
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    s.trim().chars().take(MAX_ARG_CHARS).collect()
}

/// 渲染 Runbook：校验必填参数、拒绝未声明的参数，返回完整提示词（含通用约束）。
pub fn render(rb: &Runbook, args: &HashMap<String, String>) -> Result<String, String> {
    if let Some(k) = args
        .keys()
        .find(|k| !rb.arguments.iter().any(|a| a.name == k.as_str()))
    {
        return Err(format!("技能 {} 不接受参数 {k}", rb.name));
    }
    let mut text = rb.template.to_string();
    for a in rb.arguments {
        let v = args.get(a.name).map(|v| clean(v)).unwrap_or_default();
        if a.required && v.is_empty() {
            return Err(format!("缺少必填参数 {}（{}）", a.name, a.description));
        }
        let shown = if v.is_empty() {
            "（未指定）".to_string()
        } else {
            v
        };
        text = text.replace(&format!("{{{{{}}}}}", a.name), &shown);
    }
    text.push_str(RULES);
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(kv: &[(&str, &str)]) -> HashMap<String, String> {
        kv.iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn catalog_is_consistent() {
        let mut names = std::collections::HashSet::new();
        for rb in RUNBOOKS {
            assert!(names.insert(rb.name), "duplicate {}", rb.name);
            assert!(
                rb.name.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "{}",
                rb.name
            );
            // 每个声明的参数都出现在模板中，模板中没有未声明的占位符
            for a in rb.arguments {
                assert!(
                    rb.template.contains(&format!("{{{{{}}}}}", a.name)),
                    "{} missing {{{{{}}}}}",
                    rb.name,
                    a.name
                );
            }
            let mut rest = rb.template;
            while let Some(i) = rest.find("{{") {
                let j = rest[i..].find("}}").expect("unclosed placeholder") + i;
                let key = &rest[i + 2..j];
                assert!(
                    rb.arguments.iter().any(|a| a.name == key),
                    "{} undeclared {key}",
                    rb.name
                );
                rest = &rest[j + 2..];
            }
        }
        assert!(RUNBOOKS.len() >= 7);
    }

    #[test]
    fn render_validates_and_substitutes() {
        let rb = find("container_diagnose").unwrap();
        let t = render(rb, &args(&[("container", " web\n\u{7}x ")])).unwrap();
        assert!(t.contains("「web  x」") && !t.contains("{{"), "{t}");
        assert!(t.contains("## 约束"));
        assert!(render(rb, &args(&[])).unwrap_err().contains("container"));
        assert!(render(rb, &args(&[("container", "   ")])).is_err());
        assert!(render(rb, &args(&[("container", "a"), ("x", "1")]))
            .unwrap_err()
            .contains("不接受参数 x"));

        let long = "a".repeat(500);
        let t = render(rb, &args(&[("container", &long)])).unwrap();
        assert!(
            t.contains(&"a".repeat(MAX_ARG_CHARS)) && !t.contains(&"a".repeat(MAX_ARG_CHARS + 1))
        );

        // 可选参数缺省显示「未指定」
        let t = render(
            find("error_log_triage").unwrap(),
            &args(&[("keyword", "nginx")]),
        )
        .unwrap();
        assert!(
            t.contains("关键字：nginx") && t.contains("模块前缀：（未指定）"),
            "{t}"
        );
    }
}
