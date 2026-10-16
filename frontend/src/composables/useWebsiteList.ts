import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import { useApiQuery } from '@/composables/useApiQuery'
import { useCrudPage } from '@/composables/useCrudPage'
import { queryKeys } from '@/api/queryKeys'
import {
  listWebsites,
  createWebsite,
  updateWebsite,
  deleteWebsite,
  switchWebsiteEngine,
  deployWebsite,
  type SiteType,
} from '@/api/websites'
import { listEngines } from '@/api/webServers'
import type { Website, EngineInfo } from '@/types'
import type { Page } from '@/api/generated'
import type { ApiError } from '@/api/client'

/** 网站表单载荷（编辑态由展示层持有，提交时按此形状上抛） */
export interface WebsiteFormPayload {
  name: string
  domain: string
  root_path: string
  node_id: number
  engine: string
  ssl_enabled: boolean
  proxy_enabled: boolean
  proxy_pass: string
  site_type: SiteType
}

/**
 * 网站列表数据层（Doc/23 §3.3 T2-4 视图拆分第二站）。
 *
 * 原 WebsitesView（1181 行）把「列表加载 + 分页 + 搜索过滤 + 增删改 + 引擎切换」
 * 与模板揉在同一文件；本 composable 承接网站页签的全部请求编排：
 *
 * - 列表查询（`useApiQuery`，分页参数走 `useCrudPage`，`keepPrevious` 保留上一页）；
 * - 客户端搜索 + 状态过滤（当前页数据，与原实现一致）；
 * - 新建 / 编辑 / 删除（删除带二次确认）；引擎清单懒加载与切换。
 *
 * 与展示层的边界：**表单编辑态归弹窗组件**（本地 `reactive`），此处只收
 * 「已是合法表单」的载荷并组装请求体（`id` / `status` / `created_at` 等
 * 请求态字段属数据层职责，不让展示层拼）。行为与原实现逐行对齐，无契约变更。
 * 站点绑定与伪静态、SSL 证书分别落在 `useWebsiteBinding` / `useWebsiteRewrite`
 * / `useWebsiteCertificates`（各自独立的接口族，避免单文件再度膨胀）。
 */
export function useWebsiteList() {
  const { t } = useI18n()
  const toast = useFpToast()
  const { confirmAction } = useFpConfirm()

  const crud = useCrudPage({ pageSize: 20 })
  const { total, first, pageSize, onFirst } = crud
  const submitting = ref(false)

  const search = ref('')
  const statusFilter = ref('')

  // Modernization M2：分页走统一数据获取层，切换页保留上一页数据（keepPreviousData）
  const websitesQuery = useApiQuery<Page<Website>>(
    () => queryKeys.websites.list(crud.currentPage.value, crud.pageSize.value),
    async () => {
      const res = await listWebsites(crud.currentPage.value, crud.pageSize.value)
      crud.total.value = res.data.total
      return { data: res.data }
    },
    { keepPrevious: true },
  )
  const websites = computed<Page<Website>['data']>(() => websitesQuery.data.value?.data ?? [])
  const loading = websitesQuery.loading
  const error = websitesQuery.error

  function fetch() {
    return websitesQuery.refresh()
  }

  // M9：搜索 + 状态筛选（当前页客户端过滤）
  const filteredWebsites = computed(() => {
    const kw = search.value.trim().toLowerCase()
    return websites.value.filter((w) => {
      if (statusFilter.value && w.status !== statusFilter.value) return false
      if (!kw) return true
      return (
        w.name.toLowerCase().includes(kw) ||
        w.domain.toLowerCase().includes(kw) ||
        w.root_path.toLowerCase().includes(kw)
      )
    })
  })

  /** 组装请求体：`status` 恒为 active、`proxy_pass` 仅在开启代理时上报（与原实现一致） */
  function buildPayload(id: number, form: WebsiteFormPayload): Website {
    return {
      id,
      name: form.name,
      domain: form.domain,
      root_path: form.root_path,
      node_id: form.node_id,
      status: 'active',
      engine: form.engine,
      ssl_enabled: form.ssl_enabled,
      proxy_enabled: form.proxy_enabled,
      proxy_pass: form.proxy_enabled ? form.proxy_pass : null,
      site_type: form.site_type,
      created_at: '',
    }
  }

  /** 新建；成功返回 true（调用方据此关闭弹窗） */
  async function create(form: WebsiteFormPayload): Promise<boolean> {
    submitting.value = true
    try {
      await createWebsite(buildPayload(0, form))
      toast.success(t('common.success'))
      await fetch()
      return true
    } catch {
      toast.error(t('common.failed'))
      return false
    } finally {
      submitting.value = false
    }
  }

  /** 编辑保存；成功返回 true */
  async function update(id: number, form: WebsiteFormPayload): Promise<boolean> {
    submitting.value = true
    try {
      await updateWebsite(id, buildPayload(id, form))
      toast.success(t('common.success'))
      await fetch()
      return true
    } catch {
      toast.error(t('common.failed'))
      return false
    } finally {
      submitting.value = false
    }
  }

  /** 删除（二次确认后才发请求） */
  function remove(row: Website) {
    confirmAction({
      message: t('website.deleteConfirm', { name: row.name }),
      header: t('common.confirmAction'),
      accept: async () => {
        try {
          await deleteWebsite(row.id)
          toast.success(t('common.success'))
          await fetch()
        } catch {
          toast.error(t('common.failed'))
        }
      },
    })
  }

  // ── 引擎切换：清单懒加载（只在首次打开弹窗时拉一次） ──

  const engines = ref<EngineInfo[]>([])

  async function loadEngines() {
    if (engines.value.length) return
    try {
      const res = await listEngines()
      engines.value = res.data
    } catch {
      toast.error(t('common.failed'))
    }
  }

  /** 切换站点引擎；成功返回 true */
  async function switchEngine(id: number, engine: string): Promise<boolean> {
    if (!engine) return false
    submitting.value = true
    try {
      await switchWebsiteEngine(id, engine)
      toast.success(t('common.success'))
      await fetch()
      return true
    } catch {
      toast.error(t('common.failed'))
      return false
    } finally {
      submitting.value = false
    }
  }

  /** 正在部署的站点 ID（行内按钮 loading） */
  const deployingId = ref<number | null>(null)

  /** 部署站点配置；失败时提示后端原因（引擎未安装 / 配置校验输出） */
  async function deploy(row: Website): Promise<boolean> {
    deployingId.value = row.id
    try {
      const res = await deployWebsite(row.id)
      toast.success(
        res.data.reloaded ? t('website.deploySuccess') : t('website.deployPendingReload'),
      )
      return true
    } catch (e) {
      // 4xx 携带可操作原因（引擎未安装 / nginx -t 输出 / 同名 OLS 虚拟主机）；5xx 已被服务端脱敏
      const err = e as Partial<ApiError> | null
      const reason = err?.status && err.status < 500 && err.message ? err.message : ''
      toast.error(reason ? `${t('website.deployFailed')}: ${reason}` : t('website.deployFailed'))
      return false
    } finally {
      deployingId.value = null
    }
  }

  return {
    // 列表
    websites,
    filteredWebsites,
    loading,
    error,
    total,
    first,
    pageSize,
    onFirst,
    fetch,
    // 筛选
    search,
    statusFilter,
    // 写操作
    submitting,
    create,
    update,
    remove,
    // 引擎
    engines,
    loadEngines,
    switchEngine,
    // 部署
    deployingId,
    deploy,
  }
}
