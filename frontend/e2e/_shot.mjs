import { chromium } from '@playwright/test'
import fs from 'node:fs'
const base = process.env.BASE ?? 'http://127.0.0.1:5173'
const out = process.env.OUT ?? '/tmp/shots'
const theme = process.env.THEME ?? 'dark'
fs.mkdirSync(out, { recursive: true })
const routes = ['/dashboard','/websites','/projects','/runtimes','/web-servers','/plugins','/app-store','/docker','/databases','/backups','/files','/nodes','/health','/firewall','/waf','/tasks','/scheduled-tasks','/memos','/terminal','/users','/operation-logs','/system-logs','/settings']
const b = await chromium.launch(); const ctx = await b.newContext({ viewport: { width: 1440, height: 900 }, locale: 'zh-CN' })
const p = await ctx.newPage()
await p.goto(base + '/login'); await p.waitForTimeout(800)
await p.screenshot({ path: `${out}/00-login.png` })
await p.fill('input:not([type=password])', 'flameops'); await p.fill('input[type=password]', process.env.PW ?? 'Admin12345!'); await p.click('button:has-text("登")'); await p.waitForTimeout(2500)
if (await p.locator('text=修改初始密码').count()) {
  const pws = p.locator('input[type=password]'); const n = await pws.count()
  await pws.nth(n-3).fill('Admin12345!'); await pws.nth(n-2).fill('NewPass12345!'); await pws.nth(n-1).fill('NewPass12345!')
  await p.click('button:has-text("确认")'); await p.waitForTimeout(3000); console.log('pw changed', p.url())
}
if (p.url().includes('/login')) { await p.fill('input:not([type=password])', 'flameops'); await p.fill('input[type=password]', process.env.PW); await p.click('button:has-text("登")'); await p.waitForTimeout(3000); console.log('relogin', p.url()) }
await p.evaluate((t) => { localStorage.setItem('flamepanel.mode', t); document.documentElement.classList.toggle('dark', t==='dark') }, theme)
let i = 1
async function login(){ await p.goto(base+'/login'); await p.waitForTimeout(800); await p.fill('input:not([type=password])', 'flameops'); await p.fill('input[type=password]', process.env.PW); await p.click('button:has-text("登")'); await p.waitForTimeout(2500) }
for (const r of routes) { await p.goto(base + r); await p.waitForTimeout(1800); if (p.url().includes('/login')) { console.log('kicked at', r); await login(); await p.goto(base + r); await p.waitForTimeout(1800) } await p.screenshot({ path: `${out}/${String(i++).padStart(2,'0')}-${r.slice(1)}.png` }); console.log('ok', r) }
await b.close()
