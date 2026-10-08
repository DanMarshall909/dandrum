import {test,expect} from '@playwright/test';

test('New empty patch replaces the accepted preset and returns the editor to Empty',async({page})=>{
  await page.goto('/');
  await page.getByRole('button',{name:/^Patch/}).click();
  await page.getByRole('button',{name:'Load Felt Kit',exact:true}).click();
  await expect(page.getByRole('banner').getByRole('status')).toContainText('Ready');
  await expect(page.getByRole('button',{name:'New empty patch',exact:true})).not.toBeVisible();
  await page.getByRole('button',{name:/^Patch/}).click();
  await page.getByRole('button',{name:'New empty patch',exact:true}).click();
  await expect(page.getByRole('button',{name:'Patch Untitled',exact:true})).toBeVisible();
  await expect(page.getByRole('main')).toContainText('Drop samples here');
  await expect(page.getByRole('button',{name:'Nothing to undo',exact:true})).toBeDisabled();
});

test('Reload asks before discarding edits and Cancel preserves the patch and undo',async({page})=>{
  await page.goto('/');await page.getByRole('button',{name:/^Patch/}).click();
  await page.getByRole('button',{name:'Load Felt Kit',exact:true}).click();await expect(page.getByRole('banner').getByRole('status')).toContainText('Ready');
  await page.getByRole('tab',{name:'Voice',exact:true}).click();
  const cutoff=page.getByRole('slider',{name:'Cutoff',exact:true});await cutoff.focus();await cutoff.press('ArrowUp');
  const edited=await cutoff.getAttribute('aria-valuenow');
  await page.getByRole('button',{name:/^Patch/}).click();await page.getByRole('button',{name:'Reload Patch',exact:true}).click();
  await expect(page.getByRole('dialog',{name:'Discard unsaved edits?'})).toBeVisible();
  await page.getByRole('button',{name:'Keep editing',exact:true}).click();
  await expect(cutoff).toHaveAttribute('aria-valuenow',edited);await expect(page.getByRole('button',{name:/Undo Change Cutoff/})).toBeEnabled();
  await page.getByRole('button',{name:/^Patch/}).click();await page.getByRole('button',{name:'Reload Patch',exact:true}).click();
  await page.getByRole('button',{name:'Discard and continue',exact:true}).click();
  await expect(page.getByRole('banner').getByRole('status')).toContainText('Ready');
  await expect(page.getByRole('button',{name:'Nothing to undo',exact:true})).toBeDisabled();
  await page.getByRole('tab',{name:'Voice',exact:true}).click();await expect(cutoff).toHaveAttribute('aria-valuenow','0.42');
});
