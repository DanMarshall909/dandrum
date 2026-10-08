import {test,expect} from '@playwright/test';
const load=async page=>{await page.goto('/');await page.getByRole('button',{name:/^Patch/}).click();await page.getByRole('button',{name:'Load Felt Kit',exact:true}).click();await expect(page.getByRole('banner').getByRole('status')).toContainText('Ready');};
test('every processor and add button stays exposed at minimum and default sizes',async({page})=>{
  await load(page);await page.getByRole('tab',{name:'Effects',exact:true}).click();
  for(const size of ['Min','Default']){await page.getByRole('radio',{name:size,exact:true}).click();
    for(const id of ['eq','comp','sat','eq2','crush','conv','eqr','delay','flt']){const control=page.locator('[data-module-id="'+id+'"]');await expect(control).toBeVisible();
      const exposed=await control.evaluate(e=>{const r=e.getBoundingClientRect(),p=e.parentElement.getBoundingClientRect();return r.left>=p.left-1&&r.right<=p.right+1;});expect(exposed).toBe(true);
    }
    for(const name of ['Drums','Keys','Break','Reverb','Delay'])await expect(page.getByRole('button',{name:'Add module to '+name,exact:true})).toBeVisible();
    await page.getByRole('button',{name:'Add module to Drums',exact:true}).click();await expect(page.getByRole('dialog',{name:'Add processor',exact:true})).toBeVisible();await page.keyboard.press('Escape');
  }
});
test('editing and confirming displayed envelope units preserves tiny durations and sustain, while real edits update the curve',async({page})=>{
  await load(page);await page.getByRole('tab',{name:'Voice',exact:true}).click();await page.getByRole('radio',{name:'Expanded',exact:true}).click();
  const attack=page.getByRole('spinbutton',{name:'Filter envelope Attack',exact:true});await expect(attack).toContainText('1');await expect(attack).toContainText('ms');
  await attack.click();const input=attack.getByRole('textbox');await expect(input).toHaveValue('1');await input.press('Enter');
  await expect(page.getByRole('slider',{name:'Filter envelope attack',exact:true})).toHaveAttribute('aria-valuenow','0.001');
  const sustain=page.getByRole('spinbutton',{name:'Amp envelope Sustain',exact:true});await expect(sustain).toContainText('-6.0');await sustain.click();await sustain.getByRole('textbox').press('Enter');
  await expect(page.getByRole('slider',{name:'Amp envelope sustain',exact:true})).toHaveAttribute('aria-valuenow','0.5');
  await attack.click();await attack.getByRole('textbox').fill('12');await attack.getByRole('textbox').press('Enter');
  await expect(page.getByRole('slider',{name:'Filter envelope attack',exact:true})).toHaveAttribute('aria-valuenow','0.012');
});
test('composed processor knobs share context controls and synth source menus explain asset-only actions',async({page})=>{
  await load(page);await page.getByRole('tab',{name:'Effects',exact:true}).click();await page.locator('[data-module-id="comp"]').click();
  const knob=page.getByRole('region',{name:'Compressor editor',exact:true}).getByRole('slider',{name:'Threshold',exact:true});await knob.click({button:'right'});
  await expect(page.getByRole('menuitem',{name:'Learn MIDI',exact:false})).toBeDisabled();await expect(page.getByRole('menuitem',{name:/Reset to default/})).toBeVisible();await page.keyboard.press('Escape');
  await page.getByText('Dandrum patches',{exact:true}).click();await page.locator('[data-ctx="src|module:lush"]').click({button:'right'});
  await expect(page.getByRole('menuitem',{name:/Detect transients/})).toBeDisabled();await page.getByRole('menuitem',{name:'Open in Voice',exact:true}).click();await expect(page.getByRole('tab',{name:'Voice',exact:true})).toHaveAttribute('aria-selected','true');
});
test('slice history inspector edits the same markers and mapping and bypass restores their identities',async({page})=>{
  await load(page);await page.getByRole('tab',{name:/^Slices/}).click();
  await page.locator('[data-ctx="op|amen-slices"]').click();
  const count=page.getByRole('spinbutton',{name:'Count',exact:true}).last();
  await count.click();await count.getByRole('textbox').fill('4');await count.getByRole('textbox').press('Enter');
  const rows=page.locator('[data-ctx^="slice|"]');await expect(rows).toHaveCount(4);
  await page.keyboard.press('Tab');await page.keyboard.press('Control+z');await expect(rows).toHaveCount(8);
  await page.getByRole('checkbox',{name:'Slice',exact:true}).click();await expect(rows).toHaveCount(0);
  await page.getByRole('checkbox',{name:'Slice',exact:true}).click();await expect(rows).toHaveCount(8);
  await expect(rows.nth(2)).toContainText('Snare 1');
});
test('a narrow browser preserves the native minimum editor and anchors patch menus inside it',async({page})=>{
  await page.setViewportSize({width:390,height:844});await page.goto('/');
  await page.getByRole('radio',{name:'Min',exact:true}).click();await page.evaluate(()=>window.scrollTo(0,0));
  const editor=page.getByTestId('trigger-editor');await expect(editor).toHaveCSS('width','820px');
  const box=await editor.boundingBox();expect(box.x).toBeGreaterThanOrEqual(0);
  await page.getByRole('button',{name:/^Patch/}).click();
  const menu=page.locator('.patch-menu');await expect(menu).toBeVisible();
  expect(await menu.evaluate(e=>!!e.closest('[data-testid="trigger-editor"]'))).toBe(true);
  const bounds=await menu.boundingBox(),frame=await editor.boundingBox();expect(bounds.x).toBeGreaterThanOrEqual(frame.x);expect(bounds.x+bounds.width).toBeLessThanOrEqual(frame.x+frame.width);
  await page.getByRole('button',{name:'Load Felt Kit',exact:true}).click();await expect(page.getByRole('banner').getByRole('status')).toContainText('Ready');
});
test('a cancelled shared knob gesture restores the accepted value and does not add undo',async({page})=>{
  await load(page);await page.getByRole('tab',{name:'Voice',exact:true}).click();
  const slider=page.getByRole('slider',{name:'Cutoff',exact:true}).first(),box=await slider.boundingBox();
  await page.mouse.move(box.x+box.width/2,box.y+box.height/2);await page.mouse.down();await page.mouse.move(box.x+box.width/2,box.y-60);
  await expect(slider).not.toHaveAttribute('aria-valuenow','0.42');
  await page.evaluate(()=>window.dispatchEvent(new Event('blur')));await page.mouse.up();
  await expect(slider).toHaveAttribute('aria-valuenow','0.42');await expect(page.getByRole('button',{name:'Nothing to undo',exact:true})).toBeDisabled();
});
