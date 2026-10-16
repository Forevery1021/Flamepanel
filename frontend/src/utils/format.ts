/**
 * 通用体积格式化工具——统一此前散落在 4 个视图中的重复实现。
 *
 * 规则：B 不带小数，KB 及以上保留 1 位小数（整数则省略 .0）。
 */
export function formatSize(bytes: number): string {
  if (!bytes) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let i = 0
  let v = bytes
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024
    i++
  }
  const rounded = i === 0 ? v : Math.round(v * 10) / 10
  return `${rounded} ${units[i]}`
}
