import { describe, it, expect } from 'vitest'
import {
  buildCrumbs,
  entryAction,
  entryIcon,
  formatModified,
  normalizeDir,
  pruneSelection,
  rowActionKeys,
  toggleSelection,
} from '@/utils/fileBrowser'

describe('fileBrowser utils', () => {
  it('buildCrumbs：根目录一级，多级逐段累加完整路径', () => {
    expect(buildCrumbs('/')).toEqual([{ label: '/', path: '/' }])
    expect(buildCrumbs('/var/log//nginx/')).toEqual([
      { label: '/', path: '/' },
      { label: 'var', path: '/var' },
      { label: 'log', path: '/var/log' },
      { label: 'nginx', path: '/var/log/nginx' },
    ])
  })

  it('normalizeDir：空串视为根', () => {
    expect(normalizeDir('')).toBe('/')
    expect(normalizeDir('/etc')).toBe('/etc')
  })

  it('entryIcon / entryAction：目录进入、压缩包解压、其它编辑', () => {
    expect(entryIcon({ is_dir: true, name: 'x.zip' })).toBe('oi oi-folder')
    expect(entryIcon({ is_dir: false, name: 'a.tar.gz' })).toBe('oi oi-box')
    expect(entryIcon({ is_dir: false, name: 'a.txt' })).toBe('oi oi-file')
    expect(entryAction({ is_dir: true, name: 'd' })).toBe('enter')
    expect(entryAction({ is_dir: false, name: 'b.zip' })).toBe('extract')
    expect(entryAction({ is_dir: false, name: 'c.conf' })).toBe('edit')
  })

  it('toggleSelection：去重且保持顺序', () => {
    expect(toggleSelection(['/a'], '/b', true)).toEqual(['/a', '/b'])
    const same = ['/a']
    expect(toggleSelection(same, '/a', true)).toBe(same)
    expect(toggleSelection(['/a', '/b'], '/a', false)).toEqual(['/b'])
  })

  it('pruneSelection：丢弃已不存在的路径', () => {
    expect(pruneSelection(['/a', '/gone'], [{ path: '/a' }, { path: '/c' }])).toEqual(['/a'])
  })

  it('formatModified：非法日期返回占位', () => {
    expect(formatModified('not-a-date')).toBe('—')
    expect(formatModified('2026-01-01T00:00:00Z')).not.toBe('—')
  })

  it('rowActionKeys：仅压缩包文件出现 extract，删除类在末尾', () => {
    expect(rowActionKeys({ is_dir: false, name: 'a.txt' })).toEqual([
      'rename',
      'chmod',
      'compress',
      'copy',
      'move',
      'delete',
      'purge',
    ])
    expect(rowActionKeys({ is_dir: false, name: 'a.zip' })).toContain('extract')
    expect(rowActionKeys({ is_dir: true, name: 'dir.zip' })).not.toContain('extract')
  })
})
