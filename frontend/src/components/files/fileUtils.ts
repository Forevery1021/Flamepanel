/** 文件管理视图共用的路径与错误小工具（路径均为根内相对路径，以 `/` 开头）。 */

/** 拼接目录与名称：`joinPath('/', 'a')` → `/a`，`joinPath('/a/', 'b')` → `/a/b` */
export function joinPath(dir: string, name: string): string {
  const base = dir.replace(/\/+$/, '')
  return `${base}/${name.replace(/^\/+/, '')}` || '/'
}

/** 上级目录：`/a/b` → `/a`，`/a` → `/` */
export function parentOf(path: string): string {
  const trimmed = path.replace(/\/+$/, '')
  const idx = trimmed.lastIndexOf('/')
  return idx <= 0 ? '/' : trimmed.slice(0, idx)
}

/** 用户输入的相对名称按当前目录解析，以 `/` 开头的视为根内绝对路径 */
export function resolveInput(dir: string, input: string): string {
  const v = input.trim()
  return v.startsWith('/') ? v : joinPath(dir, v)
}

const ARCHIVE_RE = /\.(zip|tar|tar\.gz|tgz|tar\.xz|txz|gz)$/i

/** 后端可解压的压缩包扩展名 */
export function isArchive(name: string): boolean {
  return ARCHIVE_RE.test(name)
}

/** 去掉压缩包扩展名：`site.tar.gz` → `site` */
export function stripArchiveExt(name: string): string {
  return name.replace(ARCHIVE_RE, '') || name
}

/** 规范化 API 错误 / AxiosError 的 HTTP 状态码 */
export function errorStatus(e: unknown): number {
  const err = e as { status?: number; response?: { status?: number } } | null
  return err?.status || err?.response?.status || 0
}

/** 触发浏览器下载一个 Blob */
export function saveBlob(blob: Blob, name: string): void {
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = name
  document.body.appendChild(a)
  a.click()
  a.remove()
  setTimeout(() => URL.revokeObjectURL(url), 1000)
}
