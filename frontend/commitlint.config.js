/**
 * commitlint 配置 — 规范 Git 提交信息（Conventional Commits）。
 *
 * P0-2：对接 CHANGELOG 自动生成，保证提交历史可读、可回溯。
 * 规则见 https://commitlint.js.org/reference/rules-conventional-commits.html
 */
export default {
  extends: ['@commitlint/config-conventional'],
  rules: {
    'type-enum': [
      2,
      'always',
      [
        'feat', // 新功能
        'fix', // 修复
        'docs', // 文档
        'refactor', // 重构
        'perf', // 性能优化
        'test', // 测试
        'build', // 构建/依赖
        'ci', // CI 配置
        'chore', // 其他
        'revert', // 回滚
        'style', // 格式
      ],
    ],
    // 标题尽量精简（进入 CHANGELOG），控制在 72 字符内
    'header-max-length': [2, 'always', 72],
    // 允许提交信息内的关联引用（Ref #11 等）
    'body-max-line-length': [0, 'always', 200],
  },
}
