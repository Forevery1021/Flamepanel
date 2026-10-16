import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useFpToast } from '@/components/ui/FpToast'
import { remoteDownloadFile, remoteListFiles, remoteUploadFile } from '@/api/nodes'
import { getErrorMessage } from '@/utils/error'
import { decodeBase64, encodeBase64, joinRemotePath, parentRemotePath } from '@/utils/nodeFormat'
import type { RemoteFileEntry, ServerNode } from '@/types'

/**
 * 节点远程文件数据层（Doc/23 §3.3 T2-4 视图拆分第七站，Stage5 远程文件）。
 *
 * 一个弹窗内的「浏览 / 进入 / 上跳 / 下载 / 上传」五件事，全部围绕 `path` 这一个
 * 状态展开——故路径的三种变换（进入子目录、上跳、拼文件名）全部下沉到
 * `utils/nodeFormat` 的纯函数里，本层只负责「变换后重新拉取」。
 *
 * ## 四条边界取舍（拆分前都是内联实现，逐条保留）
 *
 * 1. **打开即拉根目录**：`openForNode()` 里 `path` 先归 `.` 再 `load()`，
 *    否则会带着上一个节点的路径去列目录（用户看到的是「别人的目录」）；
 * 2. **失败即清空列表**（不同于列表页的「失败保留旧数据」）：目录列表是
 *    「你看到的就是当前路径的内容」，失败后留着上一个目录的文件名会让用户
 *    误以为已经切过去了——这里宁可空；
 * 3. **上跳不越过根**：`.` 之上仍是 `.`（`parentRemotePath`），不会拼出 `..`
 *    送进后端（路径穿越的请求根本不会发出）；
 * 4. **上传先本地校验**：文件名与内容任一为空即拦下（后端也会拒，但本地提示
 *    更贴近「你忘了填文件名」这个真实原因）。
 *
 * 下载走浏览器 a[download]：内容以 base64 从 `/files/download` 取回，解码后
 * 组 Blob 触发保存，并立即 `revokeObjectURL`（否则每下载一次泄一个 blob URL）。
 */
export function useNodeFiles() {
  const { t } = useI18n()
  const toast = useFpToast()

  const visible = ref(false)
  const nodeId = ref(0)
  const nodeName = ref('')
  const path = ref('.')
  const entries = ref<RemoteFileEntry[]>([])
  const uploading = ref(false)
  const uploadName = ref('')
  const uploadContent = ref('')

  /** 打开：绑定节点 + 路径归根 + 清空上传区 + 立即列目录 */
  function openForNode(row: ServerNode) {
    nodeId.value = row.id
    nodeName.value = row.name
    path.value = '.'
    entries.value = []
    uploadName.value = ''
    uploadContent.value = ''
    visible.value = true
    void load()
  }

  /** 列当前目录；失败清空（见文件头取舍 2） */
  async function load() {
    if (nodeId.value <= 0) return
    try {
      const res = await remoteListFiles(nodeId.value, path.value || '.')
      entries.value = res.data
    } catch (e: unknown) {
      toast.error(getErrorMessage(e, t('node.agentUnreachable')))
      entries.value = []
    }
  }

  /** 进入子目录并重拉 */
  function enter(name: string) {
    path.value = joinRemotePath(path.value, name)
    void load()
  }

  /** 上跳一级并重拉 */
  function goUp() {
    path.value = parentRemotePath(path.value)
    void load()
  }

  /** 下载：base64 → Blob → a[download]，用完立即释放 URL */
  async function download(name: string) {
    const full = joinRemotePath(path.value, name)
    try {
      const res = await remoteDownloadFile(nodeId.value, full)
      const blob = new Blob([decodeBase64(res.data.content_base64) as unknown as BlobPart])
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = name
      a.click()
      URL.revokeObjectURL(url)
    } catch (e: unknown) {
      toast.error(getErrorMessage(e, t('node.agentUnreachable')))
    }
  }

  /** 上传：文件名 + 内容双非空（本地先拦一道），成功后清空并重拉目录 */
  async function upload() {
    if (!uploadName.value.trim() || !uploadContent.value) {
      toast.error(t('node.fileContent'))
      return
    }
    uploading.value = true
    try {
      const full = joinRemotePath(path.value, uploadName.value.trim())
      await remoteUploadFile(nodeId.value, full, encodeBase64(uploadContent.value))
      toast.success(t('common.success'))
      uploadName.value = ''
      uploadContent.value = ''
      await load()
    } catch (e: unknown) {
      toast.error(getErrorMessage(e, t('common.failed')))
    } finally {
      uploading.value = false
    }
  }

  return {
    visible,
    nodeId,
    nodeName,
    path,
    entries,
    uploading,
    uploadName,
    uploadContent,
    openForNode,
    load,
    enter,
    goUp,
    download,
    upload,
  }
}
