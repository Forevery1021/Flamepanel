import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import {
  deleteProject,
  deployProject,
  startProject,
  stopProject,
  restartProject,
  checkProjectHealth,
  enableProjectWebhook,
} from '@/api/projects'
import { webhookAbsoluteUrl } from '@/utils/projectFormat'
import type { ProjectDto } from '@/api/generated'

/**
 * 项目生命周期动作数据层（Doc/23 §3.3 T2-4 视图拆分第六站）。
 *
 * 收敛六类行操作（部署 / 启动 / 停止 / 重启 / 健康探测 / Webhook 开关与轮换）
 * 与删除——它们是原 832 行里「按钮 + 请求 + toast + 刷新」四件事重复七遍的部分：
 *
 * - **统一容错壳**：五类「无条件成功」的动作用同一个 `withActing()`——
 *   成功 → toast + 刷新；失败 → toast 且**不刷新**（失败也刷新会用一次额外请求
 *   掩盖首因，还会把错误提示冲散在两次渲染里）；
 * - **`actingId` 按 `<动作>-<id>` 加锁**：只锁当前行的当前按钮，别的行照常可点
 *   （原实现即如此，用字符串标签而非 id，因为同一行同时有五个按钮）；
 * - **健康探测看响应体判成败**：接口 200 而 `healthy=false`（探测窗口内不健康）
 *   是这条链路的正常返回形态，按「请求成功即成功」提示会让用户以为站点没问题；
 * - **Webhook 开启 / 轮换两条路径**：已开启走二次确认（轮换令牌），未开启直接开启；
 *   令牌明文仅本次响应返回，故由本层持有并交给展示层展示 + 复制。
 *
 * 删除走二次确认（文案带项目名）。
 */
export function useProjectLifecycle() {
  const { t } = useI18n()
  const toast = useFpToast()
  const { confirmAction } = useFpConfirm()

  /** 正在进行的行操作（格式 `<动作>-<id>`，空串 = 空闲） */
  const actingId = ref('')
  const webhookToken = ref('')
  const webhookUrl = ref('')

  /**
   * 统一容错壳：成功 toast + 刷新，失败 toast 且不刷新。
   *
   * `tag` 用于锁定发起该动作的那一个按钮（`actingId === \`deploy-${p.id}\``），
   * 其余按钮与其余行不受影响。
   */
  function withActing(tag: string, fn: () => Promise<unknown>, onRefresh: () => void) {
    actingId.value = tag
    fn()
      .then(() => {
        toast.success(t('common.success'))
        onRefresh()
      })
      .catch((err) => toast.error(err, t('common.failed')))
      .finally(() => {
        actingId.value = ''
      })
  }

  function deploy(p: ProjectDto, onRefresh: () => void) {
    withActing(`deploy-${p.id}`, () => deployProject(p.id), onRefresh)
  }

  function start(p: ProjectDto, onRefresh: () => void) {
    withActing(`start-${p.id}`, () => startProject(p.id), onRefresh)
  }

  function stop(p: ProjectDto, onRefresh: () => void) {
    withActing(`stop-${p.id}`, () => stopProject(p.id), onRefresh)
  }

  function restart(p: ProjectDto, onRefresh: () => void) {
    withActing(`restart-${p.id}`, () => restartProject(p.id), onRefresh)
  }

  /** 手动健康探测（Stage A-9 · Doc/20 §5.8 ④）：成败取自响应体的 `healthy` */
  async function healthCheck(p: ProjectDto) {
    actingId.value = `health-${p.id}`
    try {
      const res = await checkProjectHealth(p.id)
      if (res.data.healthy) {
        toast.success(t('project.healthOk'))
      } else {
        toast.error(t('project.healthFail'))
      }
    } catch (err) {
      toast.error(err, t('common.failed'))
    } finally {
      actingId.value = ''
    }
  }

  /** 删除（二次确认后才发请求） */
  function remove(p: ProjectDto, onRefresh: () => void) {
    confirmAction({
      message: t('project.deleteConfirm', { name: p.name }),
      header: t('common.delete'),
      accept: async () => {
        try {
          await deleteProject(p.id)
          toast.success(t('project.deleteSuccess'))
          onRefresh()
        } catch (err) {
          toast.error(err, t('common.failed'))
        }
      },
    })
  }

  /* ── Webhook 部署通道（Stage A-7） ── */

  /** 令牌窗可见性由本层持有：开启成功即弹窗，展示层不再自行置位 */
  const webhookVisible = ref(false)

  function applyWebhookResult(data: { token: string; webhook_url: string }, onRefresh: () => void) {
    webhookToken.value = data.token
    webhookUrl.value = data.webhook_url
    webhookVisible.value = true
    toast.success(t('common.success'))
    onRefresh()
  }

  /**
   * 开启 / 重置 Webhook。
   *
   * 已开启（`deploy_type === 'webhook'`）→ 二次确认后**轮换令牌**（旧令牌即时失效）；
   * 未开启 → 直接开启。两条路径的成功处理同源（`applyWebhookResult`）。
   */
  function toggleWebhook(p: ProjectDto, onRefresh: () => void) {
    if (p.deploy_type === 'webhook') {
      confirmAction({
        message: t('project.webhookResetConfirm', { name: p.name }),
        header: t('project.webhookReset'),
        accept: async () => {
          try {
            const res = await enableProjectWebhook(p.id)
            applyWebhookResult(res.data, onRefresh)
          } catch (err) {
            toast.error(err, t('common.failed'))
          }
        },
      })
      return
    }
    enableProjectWebhook(p.id)
      .then((res) => applyWebhookResult(res.data, onRefresh))
      .catch((err) => toast.error(err, t('common.failed')))
  }

  /** 复制 Webhook 绝对地址（后端回显相对路径，CI 侧需要绝对地址） */
  async function copyWebhookUrl() {
    try {
      await navigator.clipboard.writeText(
        webhookAbsoluteUrl(window.location.origin, webhookUrl.value),
      )
      toast.success(t('common.copySuccess'))
    } catch {
      toast.error(t('common.failed'))
    }
  }

  return {
    actingId,
    webhookVisible,
    webhookToken,
    webhookUrl,
    deploy,
    start,
    stop,
    restart,
    healthCheck,
    remove,
    toggleWebhook,
    copyWebhookUrl,
  }
}
