import api from './client'
import type {
  FileInfo,
  FileWriteResult,
  FileCompressResult,
  FileExtractResult,
  RecycleItem,
} from '@/types'
import type { TransferItem, TransferMode, TransferResult } from './generated'

export type { TransferItem, TransferMode, TransferResult }

/** 长耗时文件操作（上传 / 压缩 / 解压）不受全局 15s 超时限制 */
const NO_TIMEOUT = { timeout: 0 }

export function listFiles(path: string) {
  return api.get<FileInfo[]>('/files', { params: { path } })
}

export function readFile(path: string) {
  return api.get<string>('/files/read', { params: { path } })
}

/**
 * 保存文件。`expectedModified` 为打开时看到的 `modified_at`：
 * 文件期间被他人修改时后端返回 409，由调用方确认后不带该参数强制覆盖。
 */
export function writeFile(path: string, content: string, expectedModified?: string) {
  return api.post<FileWriteResult>('/files/write', {
    path,
    content,
    expected_modified: expectedModified ?? null,
  })
}

export function createFile(path: string) {
  return api.post('/files/create-file', { path })
}

export function createDir(path: string) {
  return api.post('/files/create-dir', { path })
}

/** 删除：默认移入回收站；`permanent` = 直接删除 */
export function deleteFile(path: string, recursive = false, permanent = false) {
  return api.delete<string>('/files/delete', { params: { path, recursive, permanent } })
}

export function renameFile(oldPath: string, newPath: string) {
  return api.post('/files/rename', { old_path: oldPath, new_path: newPath })
}

export function chmodFile(path: string, mode: string) {
  return api.post('/files/chmod', { path, mode })
}

/**
 * 上传：请求体为文件原始字节（后端流式落盘），不是 multipart。
 * 同名文件存在且未指定 `overwrite` 时返回 409。
 */
export function uploadFile(
  path: string,
  file: File,
  opts: { overwrite?: boolean; onProgress?: (loaded: number, total: number) => void } = {},
) {
  return api.post('/files/upload', file, {
    ...NO_TIMEOUT,
    params: { path, name: file.name, overwrite: opts.overwrite ?? false },
    headers: { 'Content-Type': 'application/octet-stream' },
    onUploadProgress: (e) => opts.onProgress?.(e.loaded, e.total ?? file.size),
  })
}

export function downloadFile(path: string) {
  return api.get<Blob>('/files/download', {
    ...NO_TIMEOUT,
    params: { path },
    responseType: 'blob',
  })
}

/** 压缩：`dest` 扩展名决定格式（.zip / .tar.gz / .tgz） */
export function compressFiles(paths: string[], dest: string, overwrite = false) {
  return api.post<FileCompressResult>('/files/compress', { paths, dest, overwrite }, NO_TIMEOUT)
}

/** 解压：`dest` 缺省为压缩包所在目录；`overwrite` 替换同名顶层条目（旧条目进回收站） */
export function extractArchive(path: string, dest?: string, overwrite = false) {
  return api.post<FileExtractResult>(
    '/files/extract',
    { path, dest: dest || null, overwrite },
    NO_TIMEOUT,
  )
}

/**
 * 批量复制 / 移动到目录：逐项结果（部分失败仍为 200，见 `items[].ok` / `error_kind`）。
 * `overwrite` 时同名旧条目先移入回收站。
 */
export function transferFiles(
  mode: TransferMode,
  sources: string[],
  destDir: string,
  overwrite = false,
) {
  return api.post<TransferResult>(
    `/files/${mode}`,
    { sources, dest_dir: destDir, overwrite },
    NO_TIMEOUT,
  )
}

export function listRecycle() {
  return api.get<RecycleItem[]>('/files/recycle')
}

export function restoreRecycle(id: string) {
  return api.post<RecycleItem>(`/files/recycle/${encodeURIComponent(id)}/restore`)
}

export function purgeRecycle(id: string) {
  return api.delete(`/files/recycle/${encodeURIComponent(id)}`)
}

export function emptyRecycle() {
  return api.delete<{ purged: number }>('/files/recycle')
}
