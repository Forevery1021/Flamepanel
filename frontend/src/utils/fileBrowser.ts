import type { FileInfo } from '@/types'
import { isArchive } from '@/components/files/fileUtils'

/** 面包屑一项：展示文本 + 目标路径 */
export interface FileCrumb {
  label: string
  path: string
}

/** `/var/log/nginx` → `/`、`var`、`log`、`nginx` 四级面包屑（每级带完整目标路径） */
export function buildCrumbs(path: string): FileCrumb[] {
  const crumbs: FileCrumb[] = [{ label: '/', path: '/' }]
  let acc = ''
  for (const part of path.split('/').filter(Boolean)) {
    acc += '/' + part
    crumbs.push({ label: part, path: acc })
  }
  return crumbs
}

/** 规范化导航目标：空串视为根目录 */
export function normalizeDir(path: string): string {
  return path || '/'
}

/** 行图标：目录 / 压缩包 / 普通文件 */
export function entryIcon(row: Pick<FileInfo, 'is_dir' | 'name'>): string {
  if (row.is_dir) return 'oi oi-folder'
  return isArchive(row.name) ? 'oi oi-box' : 'oi oi-file'
}

/** 双击 / 点击文件名时的默认动作 */
export type EntryAction = 'enter' | 'extract' | 'edit'

export function entryAction(row: Pick<FileInfo, 'is_dir' | 'name'>): EntryAction {
  if (row.is_dir) return 'enter'
  return isArchive(row.name) ? 'extract' : 'edit'
}

/** 勾选 / 取消勾选单行（去重，保持原顺序） */
export function toggleSelection(selected: string[], path: string, on: boolean): string[] {
  if (on) return selected.includes(path) ? selected : [...selected, path]
  return selected.filter((p) => p !== path)
}

/** 列表刷新后丢弃已不存在的选中项 */
export function pruneSelection(selected: string[], entries: Pick<FileInfo, 'path'>[]): string[] {
  const paths = new Set(entries.map((e) => e.path))
  return selected.filter((p) => paths.has(p))
}

/** 本地化的修改时间；非法日期返回 `—` */
export function formatModified(iso: string): string {
  const d = new Date(iso)
  return Number.isNaN(d.getTime()) ? '—' : d.toLocaleString()
}

/** 行级「更多」菜单的动作键（顺序即菜单顺序；`extract` 仅压缩包文件出现） */
export type FileRowActionKey =
  'rename' | 'chmod' | 'compress' | 'copy' | 'move' | 'extract' | 'delete' | 'purge'

export function rowActionKeys(row: Pick<FileInfo, 'is_dir' | 'name'>): FileRowActionKey[] {
  const keys: FileRowActionKey[] = ['rename', 'chmod', 'compress', 'copy', 'move']
  if (!row.is_dir && isArchive(row.name)) keys.push('extract')
  keys.push('delete', 'purge')
  return keys
}
