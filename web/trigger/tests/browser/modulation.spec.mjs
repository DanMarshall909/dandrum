import {test,expect} from '@playwright/test';
const load=async page=>{await page.goto('/');await page.getByRole('button',{name:/^Patch/}).click();await page.getByRole('button',{name:'Load Felt Kit',exact:true}).click();await expect(page.getByRole('banner').getByRole('status')).toContainText('Ready');};
test('modulation amount, grouping, bypass and nested control menus reflect accepted routes and undo',async({page})=>{
  await load(page);await page.getByRole('tab',{name:/^Modulation/}).click();const amount=page.getByRole('slider',{name:'Filter envelope to Filter cutoff amount',exact:true});
  await amount.press('ArrowUp');await expect(amount).toHaveAttribute('aria-valuenow','0.43');await page.keyboard.press('Control+z');await expect(amount).toHaveAttribute('aria-valuenow','0.42');
  const toggle=page.getByRole('switch',{name:'Enable LFO 2 to Filter drive',exact:true});await toggle.click();await expect(toggle).toHaveAttribute('aria-checked','true');await page.keyboard.press('Control+z');await expect(toggle).toHaveAttribute('aria-checked','false');
  await page.getByRole('radio',{name:'Destination',exact:true}).click();await expect(page.getByText('Filter cutoff',{exact:true}).first()).toBeVisible();
  await page.getByRole('tab',{name:'Voice',exact:true}).click();await page.locator('[data-knob="Cutoff"]').click({button:'right'});await page.getByRole('menuitem',{name:/^Modulation/}).click();await page.getByRole('menuitem',{name:'Add',exact:true}).click();
  await page.getByRole('menuitem',{name:'LFO 2',exact:true}).click();await expect(page.getByRole('button',{name:/^Undo/})).toHaveAttribute('title',/Add route/);
  await page.getByRole('tab',{name:/^Modulation/}).click();await expect(page.locator('[data-ctx^="route|"]')).toHaveCount(15);await page.keyboard.press('Control+z');await expect(page.locator('[data-ctx^="route|"]')).toHaveCount(14);
});
test('macro inspector edits value, accepts MIDI learn, binds a destination and renames through real operations',async({page})=>{
  await load(page);await page.locator('[data-ctx="macro|Tone"]').click({button:'right'});await page.getByRole('menuitem',{name:'Edit macro',exact:true}).click();
  const value=page.getByRole('spinbutton',{name:'Value',exact:true});await value.press('ArrowUp');await expect(page.getByRole('slider',{name:'Tone',exact:true})).toHaveAttribute('aria-valuenow','0.57');
  await page.getByRole('button',{name:'Learn MIDI',exact:true}).click();await page.getByRole('button',{name:'Send MIDI CC',exact:true}).click();await expect(page.locator('aside').last()).toContainText('CC 74');
  await page.getByRole('button',{name:'Add destination…',exact:true}).click();await page.getByRole('menuitem',{name:'Pan',exact:true}).click();await expect(page.locator('aside').last()).toContainText('Pan');
  await page.getByRole('button',{name:'Rename…',exact:true}).click();await page.getByRole('textbox',{name:'Name',exact:true}).fill('Colour');await page.getByRole('button',{name:'Rename',exact:true}).click();
  await expect(page.getByRole('slider',{name:'Colour',exact:true})).toBeVisible();await page.keyboard.press('Control+z');await expect(page.getByRole('slider',{name:'Tone',exact:true})).toBeVisible();
});
