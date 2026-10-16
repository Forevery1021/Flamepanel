import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useFpToast } from '@/components/ui/FpToast'
import { useDockerResource } from '@/composables/useDockerResource'
import {
  composeDeploy,
  composeUp,
  composeDown,
  composeConvertRun,
  containerToCompose,
} from '@/api/docker'
import type { ComposeConversion } from '@/types'

/** Compose 部署载荷（表单编辑态在展示组件内，提交时上抛） */
export interface ComposeDeployPayload {
  name: string
  yaml: string
}

/** 部署表单的默认 YAML（原实现的初值，供展示层复用同一份） */
export const DEFAULT_COMPOSE_YAML = 'version: "3"\nservices:\n  web:\n    image: nginx:alpine'

/**
 * Docker Compose 数据层（Doc/23 §3.3 T2-4 视图拆分第三站）。
 *
 * 与其余四个页签的两点差异，都保留原语义：
 * 1. **部署表单的初值是一段可用的 YAML**（不是空串）——用户点开即可改，而不是
 *    对着空白框想格式；`DEFAULT_COMPOSE_YAML` 导出后由展示层复用，避免两处漂移。
 * 2. `up` / `down` **不刷新项目表**（原实现如此）：这两个动作是长操作，后端返回
 *    即代表已受理；项目状态由下次进入页签或手动刷新时体现（`status` 字段来自
 *    `docker compose ls`，prompt 刷新会读到中间态）。
 */
export function useDockerCompose() {
  const { t } = useI18n()
  const toast = useFpToast()

  /** Y3：惰性启用——首次切到 Compose 页签时由视图层置 true */
  const enabled = ref(false)
  const res = useDockerResource('compose', enabled)
  const projects = res.data
  const loading = res.loading
  const error = res.error

  const submitting = ref(false)

  /** 部署（compose up -d）；成功返回 true */
  async function deploy(payload: ComposeDeployPayload): Promise<boolean> {
    // 原实现的前置校验：项目名或 YAML 为空时给 warning 且**不发请求**
    if (!payload.name || !payload.yaml) {
      toast.warning(t('common.required'))
      return false
    }
    submitting.value = true
    try {
      await composeDeploy(payload.name, payload.yaml)
      toast.success(t('common.success'))
      await res.refresh()
      return true
    } catch (e) {
      toast.error(e, t('common.failed'))
      return false
    } finally {
      submitting.value = false
    }
  }

  /**
   * 转换（M-D：`docker run` 粘贴导入 / 存量容器收编）：结果只回填表单供用户审阅，
   * 部署仍由用户点「部署」触发——转换本身不产生任何副作用。
   */
  const converting = ref(false)

  async function convert(
    request: () => Promise<ComposeConversion>,
  ): Promise<ComposeConversion | null> {
    converting.value = true
    try {
      return await request()
    } catch (e) {
      toast.error(e, t('common.failed'))
      return null
    } finally {
      converting.value = false
    }
  }

  const convertRun = (command: string) =>
    convert(async () => (await composeConvertRun(command)).data)
  const exportContainer = (id: string) => convert(async () => (await containerToCompose(id)).data)

  async function up(name: string): Promise<boolean> {
    try {
      await composeUp(name)
      toast.success(t('common.success'))
      return true
    } catch (e) {
      toast.error(e, t('common.failed'))
      return false
    }
  }

  async function down(name: string): Promise<boolean> {
    try {
      await composeDown(name)
      toast.success(t('common.success'))
      return true
    } catch (e) {
      toast.error(e, t('common.failed'))
      return false
    }
  }

  return {
    projects,
    loading,
    error,
    enabled,
    fetch: res.refresh,
    submitting,
    deploy,
    converting,
    convertRun,
    exportContainer,
    up,
    down,
  }
}
