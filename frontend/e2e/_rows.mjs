import { chromium } from 'playwright'
const base='http://127.0.0.1:5173', out='/tmp/rows', PW=process.env.PW
const b=await chromium.launch(); const ctx=await b.newContext({viewport:{width:1440,height:900},locale:'zh-CN'}); const p=await ctx.newPage()
await p.goto(base+'/login'); await p.waitForTimeout(800)
await p.fill('input:not([type=password])','flameops'); await p.fill('input[type=password]',PW); await p.click('button:has-text("登")'); await p.waitForTimeout(2500)
// create a project via API using token in storage
const tok = await p.evaluate(()=>Object.keys(localStorage).map(k=>[k,localStorage.getItem(k)]).find(([k])=>/token/i.test(k))?.[1])
const t = (tok||'').replace(/^"|"$/g,'')
for (const [name,port] of [['blog',3000],['api',8081]]) {
  const r = await p.request.post('http://127.0.0.1:8080/api/projects',{headers:{authorization:`Bearer ${t}`},data:{name,kind:'node',source:'git',source_url:'https://example.com/x.git',port,domain:`${name}.example.com`,health_check_path:'/health'}})
  console.log('create',name,r.status())
}
await p.goto(base+'/projects'); await p.waitForTimeout(1800); await p.screenshot({path:`${out}/projects.png`})
const more = p.locator('.row-actions button[aria-haspopup="menu"]').first()
if (await more.count()) { await more.click(); await p.waitForTimeout(500); await p.screenshot({path:`${out}/projects-menu.png`}) }
await p.goto(base+'/docker'); await p.waitForTimeout(1800); await p.screenshot({path:`${out}/docker.png`})
await b.close()
