import { useConfirm } from 'openvue/useconfirm'

export interface FpConfirmOptions {
  message: string
  header?: string
  icon?: string
  acceptLabel?: string
  rejectLabel?: string
  accept?: () => void
  reject?: () => void
  /** 危险操作：红色确认按钮 */
  danger?: boolean
}

/**
 * FpConfirm — 统一确认弹窗。
 * 默认文案沿用 i18n common.* key（由调用方传入）。
 */
export function useFpConfirm() {
  const confirm = useConfirm()

  function confirmAction(options: FpConfirmOptions) {
    confirm.require({
      message: options.message,
      header: options.header ?? '确认操作',
      icon: options.icon ?? 'oi oi-exclamation-triangle',
      acceptLabel: options.acceptLabel ?? '确定',
      rejectLabel: options.rejectLabel ?? '取消',
      acceptClass: options.danger === false ? 'p-button-primary' : 'p-button-danger',
      rejectClass: 'p-button-secondary',
      accept: options.accept,
      reject: options.reject,
    })
  }

  return { confirmAction }
}
