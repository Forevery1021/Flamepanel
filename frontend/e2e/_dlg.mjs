import { chromium } from 'playwright'
const base='http://127.0.0.1:5173', out=process.env.OUT??'/tmp/dlg', PW=process.env.PW
const b=await chromium.launch(); const ctx=await b.newContext({viewport:{width:1440,height:900},locale:'zh-CN'}); const p=await ctx.newPage()
await p.goto(base+'/login'); await p.waitForTimeout(800)
await p.fill('input:not([type=password])','flameops'); await p.fill('input[type=password]',PW); await p.click('button:has-text("登")'); await p.waitForTimeout(2500)
async function shot(path, btn, name){ await p.goto(base+path); await p.waitForTimeout(1500); await p.click(btn); await p.waitForTimeout(700); await p.screenshot({path:`${out}/${name}.png`}); console.log('ok',name) }
try{ await shot('/websites','button:has-text("创建网站")','websites-create') }catch(e){console.log('x',e.message.slice(0,80))}
try{ await shot('/nodes','button:has-text("注册节点")','nodes-register') }catch(e){console.log('x',e.message.slice(0,80))}
try{ await shot('/scheduled-tasks','button:has-text("创建任务")','task-create') }catch(e){console.log('x',e.message.slice(0,80))}
try{ await shot('/backups','text=计划任务','backup-jobs'); await p.click('button:has-text("新建任务")'); await p.waitForTimeout(700); await p.screenshot({path:`${out}/backup-job-create.png`}) }catch(e){console.log('x',e.message.slice(0,80))}
try{ await p.goto(base+'/settings'); await p.waitForTimeout(1500); await p.click('text=外观'); await p.waitForTimeout(600); await p.screenshot({path:`${out}/settings-appearance.png`}); console.log('ok settings') }catch(e){console.log('x',e.message.slice(0,80))}
await b.close()
