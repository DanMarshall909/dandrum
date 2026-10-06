import {test,expect} from '@playwright/test';
test('a visible parameter edit enables named undo and restores the previous value',async({page})=>{
  await page.goto('/');
  await page.getByRole('button',{name:/^Patch/}).click();
  await page.getByRole('button',{name:'Load Felt Kit',exact:true}).click();
  await expect(page.getByRole('banner').getByRole('status')).toContainText('Ready');
  await page.getByRole('tab',{name:'Voice',exact:true}).click();
  const cutoff=page.getByRole('slider',{name:'Cutoff',exact:true});
  const initial=await cutoff.getAttribute('aria-valuenow');
  await cutoff.focus();await cutoff.press('ArrowUp');
  await expect(cutoff).not.toHaveAttribute('aria-valuenow',initial);
  const undo=page.getByRole('button',{name:/Undo Change Cutoff/});
  await expect(undo).toBeEnabled();await undo.click();
  await expect(cutoff).toHaveAttribute('aria-valuenow',initial);
  await page.keyboard.press('Control+Shift+Z');
  await expect(cutoff).not.toHaveAttribute('aria-valuenow',initial);
});
