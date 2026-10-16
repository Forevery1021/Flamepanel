/**
 * compose 模板风险预判（安装弹窗的前置提示，Doc/33 B-1）。
 *
 * 权威判定在后端结构化扫描（`security_scanner.rs`）——未勾选「确认风险」时高危项直接阻断。
 * 这里只做**提示**：与后端同口径的宽松正则，容忍多空格 / 引号 / 流式写法，
 * 让用户在提交前就知道需要勾选确认，而不是提交后才看到阻断错误。
 */
export type ComposeRisk = 'privileged' | 'dockerSock' | 'hostAccess'

const PRIVILEGED = /\bprivileged\s*:\s*["']?(true|yes|on|1)\b/i
const DOCKER_SOCK = /docker\.sock/
const HOST_NAMESPACE = /\b(network_mode|pid|ipc|userns_mode|uts)\s*:\s*["']?host\b/i
const DANGEROUS_CAP =
  /\b(CAP_)?(ALL|SYS_ADMIN|SYS_MODULE|SYS_PTRACE|SYS_RAWIO|SYS_BOOT|SYS_TIME|NET_ADMIN|DAC_READ_SEARCH|DAC_OVERRIDE|BPF|PERFMON|MAC_ADMIN|MAC_OVERRIDE)\b/
const DEVICES = /^\s*devices\s*:/m
const UNCONFINED = /\b(seccomp|apparmor)[:=]unconfined\b|label[:=]disable/i
/** 宿主敏感路径 bind 挂载（短语法 `- /etc:/x`、`- "/:/host"` 与长语法 `source: /root`）。 */
const SENSITIVE_MOUNT =
  /(^\s*-\s*["']?|source\s*:\s*["']?)(\/|~|\/(etc|root|boot|proc|sys|dev|run|var\/run|var\/lib\/docker|var\/lib\/containerd|usr|bin|sbin|lib|lib64|home))(\/[^\s:"']*)?\/?["']?(:|\s*$)/m

/** 返回最需要提示的一类风险；无风险返回 null。 */
export function detectComposeRisk(compose: string): ComposeRisk | null {
  if (!compose) return null
  if (PRIVILEGED.test(compose)) return 'privileged'
  if (DOCKER_SOCK.test(compose)) return 'dockerSock'
  const capSection = /cap_add\s*:([\s\S]*?)(\n\S|\n\s{0,4}\w+\s*:|$)/.exec(compose)?.[1] ?? ''
  if (
    HOST_NAMESPACE.test(compose) ||
    DANGEROUS_CAP.test(capSection) ||
    DEVICES.test(compose) ||
    UNCONFINED.test(compose) ||
    SENSITIVE_MOUNT.test(compose)
  ) {
    return 'hostAccess'
  }
  return null
}
