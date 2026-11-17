// Software verification cases through the real browser/HTTP/IPC/daemon seam.
// External provider fixtures cover protocol behavior, not model acceptance.
// TEST-VER-UX-101: native Enter framing and structured multiline submission.
// TEST-VER-UX-102: plain-folder state, no Git initialization, usable conversation.
// TEST-VER-UX-103: bounded expandable diagnostic in the workspace.
// TEST-VER-UX-104: border/cursor fixture and measured resize (Linux; native DPI pending).
// TEST-VER-UX-105: ordered conversation records (fixture; authenticated acceptance pending).
// TEST-VER-UX-106: streaming, permission, cancellation, rejection and process failure.
// TEST-VER-UX-107: keyboard Advanced disclosure and basic launch.
// TEST-VER-UX-108: advertised inventory, unsupported inventory, error/deadline fallback.
// TEST-VER-UX-109: defaults, provider switching and duplicate model argument validation.
// TEST-VER-UX-111: idempotency, concurrent send rejection, isolated histories/reload.
// TEST-VER-UX-110/112: native Windows/manual portions in tests/reports/qa17_acceptance_report.md.
import {test, expect, type Page} from '@playwright/test';
import {mkdir, readFile, writeFile} from 'node:fs/promises';
import path from 'node:path';
async function api(page:Page,route:string,body?:unknown) {
  return page.evaluate(async({route,body})=>{
    const r=await fetch(route,{method:body===undefined?'GET':'POST',headers:{'X-Agmux-Token':window.__AGMUX__!.token,'Content-Type':'application/json'},body:body===undefined?undefined:JSON.stringify(body)});
    return {status:r.status,value:await r.json()};
  },{route,body});
}
async function start(page:Page,title:string,cwd?:string) {
  await page.getByRole('button',{name:'New session',exact:true}).click();
  await page.getByLabel('Session name').fill(title);
  if(cwd) await page.getByLabel('Project directory').fill(cwd);
  await page.getByLabel('Agent',{exact:true}).selectOption('gemini');
  await expect(page.getByLabel('Gemini model').getByRole('option',{name:'Fixture fast',exact:true})).toHaveCount(1);
  await expect(page.getByLabel('Gemini model')).toContainText('CLI configured default (provider-default)');
  await expect(page.getByText('Arguments (JSON array)',{exact:true})).not.toBeVisible();
  await page.getByText('Advanced settings',{exact:true}).focus();
  await page.getByText('Advanced settings',{exact:true}).press('Enter');
  await expect(page.getByText('Arguments (JSON array)',{exact:true})).toBeVisible();
  await page.getByText('Advanced settings',{exact:true}).click();
  await page.getByLabel('Gemini model').selectOption('fixture-fast');
  await page.getByRole('button',{name:'Start session',exact:true}).click();
  await expect(page.locator('h1')).toHaveText(title);
  await page.getByRole('button',{name:'Chat',exact:true}).click();
  await expect(page.getByLabel('Gemini conversation')).toBeVisible();
  return (await api(page,'/api/snapshot')).value.selected as string;
}
async function send(page:Page,text:string) {
  await page.getByLabel('Message Gemini').fill(text);
  await page.getByRole('button',{name:'Send message',exact:true}).click();
}
const log=(page:Page)=>page.getByRole('log',{name:'Conversation messages'});
test.describe('QA17 public browser → real Rust daemon → external ACP fixture',()=>{
 test('TEST-VER-UX-102/103/107/108/109: plain folder, bounded error, setup and real protocol inventory',async({page})=>{
  await page.goto('/');
  const info=await api(page,'/api/info');const plain=path.join(path.dirname(info.value.cwd),'plain');await mkdir(plain);
  await start(page,'Plain project',plain);
  await send(page,'hello plain folder');await expect(log(page)).toContainText('Response 1: hello plain folder');
  await page.getByRole('button',{name:'Git',exact:true}).click();
  await expect(page.getByText('Not a Git repository',{exact:true}).first()).toBeVisible();
  await expect(page.locator('.live-error')).toHaveCount(0);
  const diff=await api(page,'/api/diff',{session_id:(await api(page,'/api/snapshot')).value.selected});
  expect(diff.value).toEqual({files:[],repository:false});
  await page.screenshot({path:path.resolve('../../../../../docs/assets/screenshots/qa17_nongit_verification.png'),fullPage:true});
  // An external long diagnostic exercises the normal UI error boundary.
  await page.route('**/api/info',route=>route.fulfill({status:400,contentType:'application/json',body:JSON.stringify({error:'Provider unavailable\n'+ 'diagnostic '.repeat(1000)})}));
  await page.getByRole('button',{name:'New session',exact:true}).click();
  await page.getByLabel('Agent',{exact:true}).selectOption('gemini');
  await page.getByRole('button',{name:'Refresh installed agents',exact:true}).click();
  await expect(page.getByRole('alert').first()).toContainText('Provider unavailable');
  expect(await page.getByRole('alert').first().evaluate(el=>el.getBoundingClientRect().height)).toBeLessThanOrEqual(180);
  await page.getByRole('button',{name:'Cancel',exact:true}).click();
 });
 test('TEST-VER-UX-101/105/106/111: two turns, multiline, duplicate/concurrent requests, reload, isolation',async({page})=>{
  await page.goto('/');const id=await start(page,'Conversation A');
  const text='first line\nsecond line';await send(page,text);
  await expect(log(page).locator('.assistant')).toContainText('Response 1: '+text);
  await expect(page.getByLabel('Conversation status')).toHaveText('idle');
  await send(page,'remember the first turn');await expect(log(page).locator('.assistant').last()).toContainText('Response 2: remember the first turn');
  const payload={session_id:id,operation:'send',request_id:'same-request',message:'exactly once'};
  expect((await api(page,'/api/chat',payload)).status).toBe(200);
  expect((await api(page,'/api/chat',payload)).status).toBe(200);
  const concurrent=await api(page,'/api/chat',{...payload,request_id:'other',message:'should not send'});expect(concurrent.status).not.toBe(200);
  await expect(log(page)).toContainText('Response 3: exactly once');
  await page.reload();await expect(log(page).locator('.user')).toHaveCount(3);await expect(log(page).locator('.assistant')).toHaveCount(3);
  expect(await log(page).textContent()).not.toContain('Native CLI fixture');
  const root=(await api(page,'/api/info')).value.cwd;
  const records=(await readFile(path.join(root,'acp-requests.jsonl'),'utf8')).trim().split('\n').map(line=>JSON.parse(line));
  expect(records.filter(r=>r.text==='exactly once')).toHaveLength(1);
  expect(records.find(r=>r.text===text)?.args).toContain('fixture-fast');
  const second=await start(page,'Conversation B');await send(page,'separate session');await expect(log(page)).toContainText('Response 1: separate session');
  expect(await log(page).textContent()).not.toContain('remember the first turn');
  await api(page,'/api/select',{session_id:id});await page.reload();await expect(log(page)).toContainText('remember the first turn');expect(await log(page).textContent()).not.toContain('separate session');
  await page.screenshot({path:path.resolve('../../../../../docs/assets/screenshots/qa17_chat_verification.png'),fullPage:true});
  await api(page,'/api/freeze',{session_id:second});
 });
 test('TEST-VER-UX-106: explicit permission, cancel and provider exit cannot remain Working',async({page})=>{
  await page.goto('/');await start(page,'Lifecycle');
  await send(page,'ask-permission');await expect(page.getByLabel('Tool permission')).toBeVisible();
  await page.getByRole('button',{name:'Reject',exact:true}).click();await expect(log(page)).toContainText('Permission: reject-once');await expect(page.getByLabel('Conversation status')).toHaveText('idle');
  await send(page,'wait-for-cancel');await expect(page.getByRole('button',{name:'Stop response'})).toBeVisible();await page.getByRole('button',{name:'Stop response'}).click();await expect(page.getByLabel('Conversation status')).toHaveText('cancelled');
  await send(page,'after stop');await expect(log(page)).toContainText('Response 3: after stop');
  await start(page,'Provider rejection');await send(page,'reject-provider');await expect(page.getByLabel('Conversation status')).toHaveText('failed');await expect(page.getByRole('alert').first()).toContainText('Provider rejected this request');
  await start(page,'Unresponsive Stop');await send(page,'ignore-cancel');await expect(page.getByRole('button',{name:'Stop response'})).toBeVisible();await page.getByRole('button',{name:'Stop response'}).click();await expect(page.getByLabel('Conversation status')).toHaveText('failed');
  await start(page,'Provider exit');await send(page,'exit-provider');await expect(page.getByLabel('Conversation status')).toHaveText('failed');await expect(page.getByRole('alert').first()).toContainText('Gemini CLI disconnected');
 });
 test('TEST-VER-UX-107/109: duplicate model flags, provider switching and discovery fallback',async({page})=>{
  await page.goto('/');await page.getByRole('button',{name:'New session',exact:true}).click();
  await page.getByLabel('Agent',{exact:true}).selectOption('gemini');
  await expect(page.getByLabel('Gemini model').getByRole('option',{name:'Fixture fast',exact:true})).toHaveCount(1);
  await page.getByLabel('Gemini model').selectOption('fixture-fast');await page.getByText('Advanced settings',{exact:true}).click();
  await page.getByLabel('Arguments (JSON array)').fill('["--model=other"]');await page.getByRole('button',{name:'Start session',exact:true}).click();
  await expect(page.getByRole('dialog').getByRole('alert')).toContainText('Set the Gemini model in either Model or Arguments');
  await page.getByLabel('Arguments (JSON array)').fill('[]');
  await page.getByLabel('Agent',{exact:true}).selectOption('shell');await expect(page.getByLabel('Gemini model')).toHaveCount(0);
  await page.route('**/api/models',route=>route.fulfill({status:503,contentType:'application/json',body:JSON.stringify({error:'Provider requires sign-in'})}));
  await page.getByLabel('Agent',{exact:true}).selectOption('gemini');
  await expect(page.getByLabel('Gemini model')).toHaveValue('');await expect(page.getByText('Using CLI configured default',{exact:true})).toBeVisible();
  await expect(page.getByLabel('Gemini model').getByRole('option')).toHaveCount(1);
  await expect(page.getByRole('button',{name:'Start session',exact:true})).toBeEnabled();
  await page.screenshot({path:path.resolve('../../../../../docs/assets/screenshots/qa17_setup_verification.png'),fullPage:true});
  await page.getByLabel('Session name').fill('Default fallback');
  await page.getByRole('button',{name:'Start session',exact:true}).click();
  await expect(page.locator('h1')).toHaveText('Default fallback');
  const id=(await api(page,'/api/snapshot')).value.selected;
  expect((await api(page,'/api/chat',{session_id:id,operation:'state'})).value.model).toBeNull();
 });
 test('TEST-VER-UX-108: unsupported inventory and a silent provider have bounded honest failures',async({page})=>{
  test.setTimeout(110000);
  await page.goto('/');const root=path.dirname((await api(page,'/api/info')).value.cwd);
  const unsupported=path.join(root,'unsupported-models'), hanging=path.join(root,'hanging-models');
  await mkdir(unsupported);await mkdir(hanging);await writeFile(path.join(unsupported,'no-model-inventory'),'');await writeFile(path.join(hanging,'hang-initialize'),'');
  const absent=await api(page,'/api/models',{cwd:unsupported});expect(absent.status).toBe(200);expect(absent.value).toBeNull();
  const before=Date.now();const timedOut=await api(page,'/api/models',{cwd:hanging});
  expect(timedOut.status).not.toBe(200);expect(timedOut.value.error).toContain('timed out');expect(Date.now()-before).toBeLessThan(100000);
 });
 test('TEST-VER-UX-104: native terminal borders and resize use cell geometry',async({page})=>{
  await page.goto('/');await start(page,'Terminal geometry');await page.getByRole('button',{name:'Terminal',exact:true}).click();
  await expect(page.locator('.xterm-screen')).toBeVisible();
  await expect.poll(async()=> (await api(page,'/api/snapshot')).value.screen).toContain('┌──────────┐');
  const id=(await api(page,'/api/snapshot')).value.selected;
  await api(page,'/api/reply',{session_id:id,message:'one\ncontinued'});
  await api(page,'/api/reply',{session_id:id,message:'two'});
  await expect.poll(async()=> (await api(page,'/api/snapshot')).value.screen).toContain('1b5b3230307e6f6e650a636f6e74696e7565641b5b3230317e0d');
  await expect.poll(async()=> (await api(page,'/api/snapshot')).value.screen).toContain('1b5b3230307e74776f1b5b3230317e0d');
  await page.setViewportSize({width:1100,height:800});await page.waitForTimeout(250);
  const box=await page.locator('.xterm-screen').boundingBox();expect(box!.height).toBeLessThan(800);expect(box!.width).toBeLessThan(1100);
  await page.screenshot({path:path.resolve('../../../../../docs/assets/screenshots/qa17_terminal_verification.png'),fullPage:true});
 });
});
