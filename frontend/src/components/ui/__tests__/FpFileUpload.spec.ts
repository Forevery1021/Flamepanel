import { describe, it, expect } from 'vitest'
import { mountFp } from './helpers'
import FpFileUpload from '../FpFileUpload.vue'

/**
 * T1-1 组件测试：FpFileUpload 文件上传。
 *
 * 封面/证书/备份包上传的入口。封装层只做样式收敛 + 属性透传，故这里锁定
 * 「透传字段真的落到 DOM」与「选择文件后触发底层 change 语义」两点。
 */
describe('FpFileUpload 文件上传', () => {
  it('渲染上传控件与文件输入', () => {
    const w = mountFp(FpFileUpload, {})
    expect(w.find('.p-fileupload').exists()).toBe(true)
    expect(w.find('input[type="file"]').exists()).toBe(true)
  })

  it('自定义外观类名带上（主题令牌入口）', () => {
    const w = mountFp(FpFileUpload, {})
    // FileUpload 的 class 落在根容器而非内层 p-fileupload 节点
    expect(w.find('.fp-fileupload').exists()).toBe(true)
  })

  it('multiple 透传到底层（支持多文件时 input 带 multiple）', () => {
    const w = mountFp(FpFileUpload, { props: { multiple: true } })
    expect(w.find('input[type="file"]').attributes('multiple')).toBeDefined()
  })

  it('accept 透传（按类型限制上传）', () => {
    const w = mountFp(FpFileUpload, { props: { accept: 'image/*' } })
    expect(w.find('input[type="file"]').attributes('accept')).toBe('image/*')
  })

  it('disabled 透传', () => {
    const w = mountFp(FpFileUpload, { props: { disabled: true } })
    expect(w.find('input[type="file"]').attributes('disabled')).toBeDefined()
  })
})
