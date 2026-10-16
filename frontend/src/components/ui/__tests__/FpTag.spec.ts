import { describe, it, expect } from 'vitest'
import { mountFp } from './helpers'
import FpTag from '../FpTag.vue'

/**
 * T1-1 组件测试：FpTag 状态徽标。
 *
 * 语义 severity → 底层标签等级的映射是运维面板的「状态语义色」入口
 * （running 绿 / 异常红 / 停用灰），映射写错会让状态视觉误导，故逐档锁定。
 * `dot` 用于运行态指示点，其类名跟随映射后的等级（避免点与底色脱节）。
 */
describe('FpTag 状态徽标', () => {
  it('渲染 value 文本', () => {
    const w = mountFp(FpTag, { props: { value: '运行中' } })
    expect(w.text()).toContain('运行中')
  })

  it('success → 成功态样式（运行中/已连接）', () => {
    const w = mountFp(FpTag, { props: { value: 'x', severity: 'success' } })
    expect(w.find('.fp-tag').classes().join(' ')).toContain('success')
  })

  it('warning → warn 等级（底层库命名为 warn 而非 warning）', () => {
    const w = mountFp(FpTag, { props: { value: 'x', severity: 'warning' } })
    expect(w.find('.fp-tag').classes().join(' ')).toContain('warn')
  })

  it('danger → 危险态样式', () => {
    const w = mountFp(FpTag, { props: { value: 'x', severity: 'danger' } })
    expect(w.find('.fp-tag').classes().join(' ')).toContain('danger')
  })

  it('info / neutral 各自映射', () => {
    expect(
      mountFp(FpTag, { props: { value: 'x', severity: 'info' } })
        .find('.fp-tag')
        .classes()
        .join(' '),
    ).toContain('info')
    expect(
      mountFp(FpTag, { props: { value: 'x', severity: 'neutral' } })
        .find('.fp-tag')
        .classes()
        .join(' '),
    ).toContain('secondary')
  })

  it('默认 severity=neutral（中性兜底，不用危险色误导）', () => {
    const w = mountFp(FpTag, { props: { value: 'x' } })
    expect(w.find('.fp-tag').classes().join(' ')).toContain('secondary')
  })

  it('dot=true：渲染指示点且等级与容器一致', () => {
    const w = mountFp(FpTag, { props: { value: 'x', severity: 'danger', dot: true } })
    const dot = w.find('.fp-tag__dot')
    expect(dot.exists()).toBe(true)
    expect(dot.classes()).toContain('is-danger')
  })

  it('dot=false 且无插槽：不渲染指示点', () => {
    const w = mountFp(FpTag, { props: { value: 'x' } })
    expect(w.find('.fp-tag__dot').exists()).toBe(false)
  })

  it('默认插槽内容优先（需要复合内容时用插槽）', () => {
    const w = mountFp(FpTag, {
      props: { value: 'ignored' },
      slots: { default: '<b class="rich">富文本</b>' },
    })
    expect(w.find('.rich').text()).toBe('富文本')
  })

  it('rounded 默认开启，可关闭', () => {
    expect(
      mountFp(FpTag, { props: { value: 'x' } })
        .find('.fp-tag')
        .classes()
        .join(' '),
    ).toContain('rounded')
    expect(
      mountFp(FpTag, { props: { value: 'x', rounded: false } })
        .find('.fp-tag')
        .classes()
        .join(' '),
    ).not.toContain('rounded')
  })
})
