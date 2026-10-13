import {test,expect} from '@playwright/test';
import {writeFile} from 'node:fs/promises';
test.beforeEach(async({page})=>{await page.coverage.startJSCoverage({resetOnNavigation:false});});
test.afterEach(async({page},testInfo)=>{
  const coverage=(await page.coverage.stopJSCoverage()).filter(entry=>entry.url.includes('/components/LogDrawer.jsx'));
  const path=testInfo.outputPath('log-drawer-coverage.json');await writeFile(path,JSON.stringify(coverage));
  await testInfo.attach('log-drawer-coverage',{path,contentType:'application/json'});
});

async function openLog(page){
  await page.goto('/');
  await page.getByRole('button',{name:/^Patch/}).click();
  await page.getByRole('button',{name:'Load Felt Kit',exact:true}).click();
  await expect(page.getByRole('banner').getByRole('status')).toContainText('Ready');
  await page.keyboard.press('a');
  await page.getByRole('button',{name:'Log',exact:true}).click();
  await expect(page.getByRole('log')).toContainText('telemetry');
}

test('event log inclusion matches names, kinds and payload text without changing recorded rows',async({page})=>{
  await openLog(page);
  const include=page.getByRole('textbox',{name:'Filter event log'}),log=page.getByRole('log');
  await include.fill('nOtEoN');
  await expect(log).toContainText('noteOn');
  await expect(log).not.toContainText('telemetry');
  await include.fill('CALL');
  await expect(log).toContainText('noteOn');
  await expect(log).not.toContainText('telemetry');
  await include.fill('"MAX":32');
  await expect(log).toContainText('telemetry');
  await expect(log).not.toContainText('noteOn');
  await include.fill('');
  await expect(log).toContainText('noteOn');
  await expect(log).toContainText('telemetry');
});

test('event log exclusions hide matching rows, combine with inclusion and reveal retained events when cleared',async({page})=>{
  await openLog(page);
  const include=page.getByRole('textbox',{name:'Filter event log'}),exclude=page.getByRole('textbox',{name:'Exclude from event log'}),log=page.getByRole('log');
  await expect(exclude).toBeVisible();
  for(const text of ['tElEmEtRy','EVENT','"MAX":32','"notes":[']){
    await exclude.fill(text);
    await expect(log).not.toContainText('telemetry');
    await expect(log).toContainText('noteOn');
  }
  await exclude.fill('no such event');
  await expect(log).toContainText('telemetry');
  await include.fill('NOTE');await exclude.fill('NOTEON');
  await expect(log).toContainText('noteOff');
  await expect(log).not.toContainText('noteOn');
  await exclude.fill('note');
  await expect(log.locator('.log-row')).toHaveCount(0);
  await exclude.fill('');
  await expect(log).toContainText('noteOn');
  await expect(log).toContainText('noteOff');
  await include.fill('');await exclude.fill('telemetry');
  await page.getByRole('switch',{name:'Host automation',exact:true}).click();
  await expect(log).toContainText('setMockSwitch');
  await expect(log).not.toContainText('telemetry');
  await exclude.fill('');
  await expect(log).toContainText('telemetry');
  await expect(log).toContainText('setMockSwitch');
});

test('event log filters stay reachable within each editor width and Clear removes recorded matches',async({page},testInfo)=>{
  await openLog(page);
  const exclude=page.getByRole('textbox',{name:'Exclude from event log'}),include=page.getByRole('textbox',{name:'Filter event log'}),log=page.getByRole('log');
  await exclude.fill('telemetry');
  for(const size of ['Min','Expanded','Default']){
    await page.getByRole('radio',{name:size,exact:true}).click();
    const drawer=page.getByRole('region',{name:'Engine event log'}),bounds=await drawer.boundingBox();
    for(const field of [include,exclude]){
      await expect(field).toBeVisible();const box=await field.boundingBox();
      expect(box.x).toBeGreaterThanOrEqual(bounds.x);expect(box.x+box.width).toBeLessThanOrEqual(bounds.x+bounds.width);
      expect(box.y+box.height).toBeLessThanOrEqual(bounds.y+bounds.height);
    }
    await expect(log).not.toContainText('telemetry');
    await page.screenshot({path:testInfo.outputPath('event-log-'+size.toLowerCase()+'.png'),animations:'disabled'});
  }
  await include.fill('note');await expect(log).toContainText('noteOn');
  await page.getByRole('button',{name:'Clear',exact:true}).click();
  await expect(log.locator('.log-row')).toHaveCount(0);
  await expect(exclude).toHaveValue('telemetry');
  await expect(include).toHaveValue('note');
});
