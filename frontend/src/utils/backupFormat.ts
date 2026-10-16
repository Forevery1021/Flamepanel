/**
 * 备份域的展示与判定纯函数（Doc/23 §3.3 T2-4 视图拆分第四站）。
 *
 * 这些映射原先散落在 905 行的 BackupView 内联实现里，被三张表（本地备份 /
 * 账号 / 任务）、三个弹窗（云端恢复 / 账号 / 任务）与工具栏多处共用。抽成纯函数后
 * 既可单测，也避免拆分子组件时「复制多份、改一处漏两处」——原文件里
 * `testTag` / `jobStatusTag` 都只写了一份，拆分时若各组件各抄一份就正好会踩这个坑。
 */

import type { BackupAccountDto, BackupJobDto } from '@/api/generated'

/** FpTag 支持的语义色（与 FpTag 的 severity 取值同源，避免各调用点各写字面量） */
export type BackupTagSeverity = 'success' | 'warning' | 'danger' | 'info' | 'neutral'

/**
 * 账号连通性测试结果 → 标签配色：成功绿 / 失败红 / 其余（含未测试）中性。
 *
 * 未测试**不猜**成失败：`last_test_status` 为 null 只表示「从未测过」，
 * 把它染红会让新账号一进门就背上一次不存在的失败。
 */
export function testStatusTag(status: string | null): BackupTagSeverity {
  if (status === 'success') return 'success'
  if (status === 'failed') return 'danger'
  return 'neutral'
}

/** 账号连通性测试结果 → 文案（null 回落「未测试」） */
export function testStatusLabel(status: string | null, t: (key: string) => string): string {
  if (status === 'success') return t('backup.testSuccess')
  if (status === 'failed') return t('backup.testFailed')
  return t('backup.testNever')
}

/** 任务上次执行状态 → 标签配色（never 与未知值同为中性） */
export function jobStatusTag(status: string): BackupTagSeverity {
  if (status === 'success') return 'success'
  if (status === 'failed') return 'danger'
  return 'neutral'
}

/** 任务上次执行状态 → 文案 */
export function jobStatusLabel(status: string, t: (key: string) => string): string {
  if (status === 'success') return t('backup.jobStatusSuccess')
  if (status === 'failed') return t('backup.jobStatusFailed')
  return t('backup.jobStatusNever')
}

/**
 * 任务备份范围 → 文案：app 范围把 key 一并带出。
 *
 * 只显示「应用目录」而不显示是哪个应用，等于让每行都写着「某处」——
 * `scope_key` 缺失时回落 `-`（而不是省略，省略会让行高与列语义随数据变化）。
 */
export function jobScopeLabel(row: BackupJobDto, t: (key: string) => string): string {
  return row.scope === 'app'
    ? `${t('backup.scopeApp')}: ${row.scope_key ?? '-'}`
    : t('backup.scopePanel')
}

/**
 * 可还原对象：面板库 `.db` / 应用数据 `app/<key>/*.tar.gz`。
 *
 * 其它 key（如手工上传的任意文件）在远端对象表里**可见但按钮禁用**——后端只受理
 * 这两类；判据与后端 `restore-cloud` 的受理范围同源，不能只判前缀（`app/` 下也
 * 可能有非 tar.gz 的对象）。
 */
export function isRestorableKey(key: string): boolean {
  return key.endsWith('.db') || (key.startsWith('app/') && key.endsWith('.tar.gz'))
}

/** 远端对象的还原范围标签（与 `isRestorableKey` 同源判前缀） */
export function restoreScopeLabel(key: string, t: (key: string) => string): string {
  return key.startsWith('app/') ? t('backup.scopeApp') : t('backup.scopePanel')
}

/** 时间展示：无值一律 `-`（表格里的空单元格会被读成「加载失败」） */
export function formatDateTime(iso: string | null | undefined): string {
  return iso ? new Date(iso).toLocaleString() : '-'
}

/** 账号类型候选：值是后端协议字段，标签只有本地/挂载盘需要翻译 */
export function accountTypeOptions(t: (key: string) => string) {
  return [
    { label: 'S3', value: 's3' },
    { label: 'SFTP', value: 'sftp' },
    { label: t('backup.accountTypeLocal'), value: 'local' },
  ]
}

/**
 * 每种账号类型的连接配置样例（JSON 字符串）。
 *
 * 样例就是「填空模板」：给出字段名比一句「请填写 JSON」有用得多；未知类型回落空串
 * （宁可不提示，也不给一个错的模板）。
 */
export function accountConfigPlaceholder(type: string): string {
  const map: Record<string, string> = {
    s3: '{"bucket":"my-bucket","endpoint":"https://s3.example.com","access_key":"...","secret_key":"..."}',
    sftp: '{"host":"10.0.0.2","port":22,"username":"root","password":"..."}',
    local: '{"path":"/mnt/backup"}',
  }
  return map[type] ?? ''
}

/**
 * 目标账号下拉候选：**只列启用账号**（停用账号建任务必然失败，列出来是误导），
 * 标签带类型后缀——同名不同型的账号（`oss` 的 S3 与 SFTP 各一个）在这里必须可分辨。
 */
export function accountSelectOptions(accounts: BackupAccountDto[]) {
  return accounts
    .filter((a) => a.enabled)
    .map((a) => ({ label: `${a.name} (${String(a.account_type).toUpperCase()})`, value: a.id }))
}

/**
 * 账号名查表：任务表的目标列由账号缓存派生，账号被删后回落 `#id`。
 *
 * 回落不是「兜底好看」——它同时暴露了数据不一致（任务指向一个不存在的账号），
 * 渲染成空白反而会让这一行看起来正常。
 */
export function accountNameOf(accounts: BackupAccountDto[], id: number): string {
  const acc = accounts.find((a) => a.id === id)
  return acc ? acc.name : `#${id}`
}
