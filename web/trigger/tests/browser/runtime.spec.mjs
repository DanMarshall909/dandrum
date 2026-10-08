import {test,expect} from '@playwright/test';

test('computer keys drive silent voice telemetry and the event drawer switches and filter work',async({page})=>{
  const errors=[];page.on('pageerror',error=>errors.push(error.message));
  await page.goto('/');await page.getByRole('button',{name:/^Patch/}).click();
  await page.getByRole('button',{name:'Load Felt Kit',exact:true}).click();await expect(page.getByRole('banner').getByRole('status')).toContainText('Ready');
  await page.keyboard.down('a');await expect(page.getByRole('banner')).toContainText('Voices 1 / 32');
  await page.keyboard.up('a');await expect(page.getByRole('banner')).toContainText('Voices 0 / 32');
  await page.keyboard.press('Control+5');await expect(page.getByRole('tab',{name:'Voice',exact:true})).toHaveAttribute('aria-selected','true');
  await page.getByRole('button',{name:'Log',exact:true}).click();
  await expect(page.getByRole('region',{name:'Engine event log'})).toBeVisible();
  const filter=page.getByRole('textbox',{name:'Filter event log'});await filter.fill('noteOn');
  await expect(page.getByRole('log')).toContainText('noteOn');
  await filter.fill('');await page.getByRole('switch',{name:'Mark file missing',exact:true}).click();
  await expect(page.getByRole('log')).toContainText('setMockSwitch');
  await page.getByRole('switch',{name:'Host automation',exact:true}).click();
  const space=page.getByRole('slider',{name:'Space',exact:true}),value=await space.getAttribute('aria-valuenow');
  await expect(space).not.toHaveAttribute('aria-valuenow',value);
  expect(errors).toEqual([]);
});
