import {test,expect} from '@playwright/test';
const load=async page=>{await page.goto('/');await page.getByRole('button',{name:/^Patch/}).click();await page.getByRole('button',{name:'Load Felt Kit',exact:true}).click();await expect(page.getByRole('banner').getByRole('status')).toContainText('Ready');};
test('transient results require explicit apply, undo restores eight slices and failed or cancelled jobs preserve them',async({page})=>{
  await load(page);await page.getByRole('tab',{name:/^Slices/}).click();
  const rows=page.locator('[data-ctx^="slice|"]');await expect(rows).toHaveCount(8);
  await page.getByRole('button',{name:'Detect transients',exact:true}).click();await expect(page.getByText(/Detecting transients/)).toBeVisible();await expect(rows).toHaveCount(8);
  const apply=page.getByRole('button',{name:/Apply as \d+ slices/});await expect(apply).toBeVisible();await expect(rows).toHaveCount(8);
  const count=Number((await apply.innerText()).match(/\d+/)[0]);await apply.click();await expect(rows).toHaveCount(count);
  await page.keyboard.press('Control+z');await expect(rows).toHaveCount(8);
  await page.getByRole('button',{name:'Log',exact:true}).click();await page.getByRole('switch',{name:'Fail next analysis',exact:true}).click();
  await page.getByRole('button',{name:'Detect transients',exact:true}).click();await expect(page.getByText('Transient detection failed',{exact:true})).toBeVisible();await expect(rows).toHaveCount(8);
  await page.getByRole('button',{name:'Detect transients',exact:true}).click();await page.getByRole('button',{name:'Cancel analysis',exact:true}).click();await expect(page.getByText(/Detecting transients/)).toHaveCount(0);await expect(rows).toHaveCount(8);
});
test('slice selection, marker editing, bulk properties and sample history use real undoable operations',async({page})=>{
  await load(page);await page.getByRole('checkbox',{name:'Trim',exact:true}).click();await expect(page.getByRole('slider',{name:'Region start',exact:true})).toHaveAttribute('aria-valuenow','0');
  await page.keyboard.press('Control+z');await expect(page.getByRole('slider',{name:'Region start',exact:true})).toHaveAttribute('aria-valuenow','0.012');
  await page.getByRole('tab',{name:/^Slices/}).click();const marker=page.getByRole('slider',{name:'Slice marker 3',exact:true});await marker.press('PageUp');await expect(marker).toHaveAttribute('aria-valuenow','0.26');
  await page.keyboard.press('Control+z');await expect(marker).toHaveAttribute('aria-valuenow','0.25');
  const rows=page.locator('[data-ctx^="slice|"]');await rows.nth(2).click();await rows.nth(6).click({modifiers:['Control']});await expect(page.getByText('2 selected',{exact:true})).toBeVisible();
  const gain=page.getByRole('spinbutton',{name:'Gain',exact:true});await gain.click();await gain.getByRole('textbox').fill('-9');await gain.getByRole('textbox').press('Enter');await expect(rows.nth(2)).toContainText('-9.0 dB');await expect(rows.nth(6)).toContainText('-9.0 dB');
  await page.keyboard.press('Tab');await page.keyboard.press('Control+z');await expect(rows.nth(2)).toContainText('0.0 dB');
});
