import { describe, it, expect } from 'vitest'
import {
  errorStatus,
  isArchive,
  joinPath,
  parentOf,
  resolveInput,
  stripArchiveExt,
} from '@/components/files/fileUtils'

describe('fileUtils · 根内相对路径', () => {
  it('joinPath 处理根目录与多余斜杠', () => {
    expect(joinPath('/', 'a.txt')).toBe('/a.txt')
    expect(joinPath('/www/', 'site')).toBe('/www/site')
    expect(joinPath('/www', '/site')).toBe('/www/site')
  })

  it('parentOf 到根为止', () => {
    expect(parentOf('/a/b/c')).toBe('/a/b')
    expect(parentOf('/a')).toBe('/')
    expect(parentOf('/')).toBe('/')
    expect(parentOf('/a/b/')).toBe('/a')
  })

  it('resolveInput：以 / 开头视为根内绝对路径，否则相对当前目录', () => {
    expect(resolveInput('/www', ' conf ')).toBe('/www/conf')
    expect(resolveInput('/www', '/etc')).toBe('/etc')
  })
})

describe('fileUtils · 压缩包识别', () => {
  it('识别后端可解压的扩展名（大小写不敏感）', () => {
    for (const n of ['a.zip', 'a.tar', 'a.tar.gz', 'a.TGZ', 'a.tar.xz', 'a.txz', 'a.gz']) {
      expect(isArchive(n), n).toBe(true)
    }
    for (const n of ['a.rar', 'a.7z', 'zip', 'a.gzip']) {
      expect(isArchive(n), n).toBe(false)
    }
  })

  it('stripArchiveExt 去掉复合扩展名', () => {
    expect(stripArchiveExt('site.tar.gz')).toBe('site')
    expect(stripArchiveExt('site.zip')).toBe('site')
    expect(stripArchiveExt('.zip')).toBe('.zip')
  })
})

describe('fileUtils · errorStatus', () => {
  it('同时兼容规范化 ApiError 与原始 AxiosError', () => {
    expect(errorStatus({ status: 409 })).toBe(409)
    expect(errorStatus({ response: { status: 403 } })).toBe(403)
    expect(errorStatus(new Error('x'))).toBe(0)
    expect(errorStatus(null)).toBe(0)
  })
})
