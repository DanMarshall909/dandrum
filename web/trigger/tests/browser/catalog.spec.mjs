import {test,expect} from './fixtures.mjs';
test('every public component renders independently in all nine catalog states, with editable props and width/background controls',async({page})=>{
 test.setTimeout(120000);const errors=[];page.on('pageerror',e=>errors.push(e.message));await page.goto('/catalog.html');
 const nav=page.locator('.catalog-nav');await expect(nav.getByRole('button').first()).toBeVisible();const names=await nav.getByRole('button').allTextContents();expect(names.length).toBeGreaterThan(90);
 for(const name of names){await nav.getByRole('button',{name,exact:true}).click();for(const state of ['default','hover','selected','disabled','empty','loading','error','long text','minimum width']){
  await page.getByRole('region',{name:'Story controls'}).locator('select').first().selectOption(state);await expect(page.locator('.catalog-preview')).not.toContainText('name.trim is not a function');await expect(page.locator('.catalog-preview [data-story-error]')).toHaveCount(0);
 }}
 await nav.getByRole('button',{name:'ParameterKnob',exact:true}).click();await page.getByRole('region',{name:'Story controls'}).locator('select').first().selectOption('default');
 await page.getByLabel('Label',{exact:true}).fill('Rate preview');await expect(page.locator('.catalog-preview').getByRole('slider',{name:'Rate preview',exact:true})).toBeVisible();
 await page.getByRole('radio',{name:'820',exact:true}).click();await expect(page.locator('[data-story-canvas]')).toHaveCSS('width','820px');await page.getByRole('radio',{name:'paper',exact:true}).click();await expect(page.locator('[data-story-canvas]')).toHaveClass(/paper/);expect(errors).toEqual([]);
});
test('dragging the catalog parameter knob updates its displayed value',async({page})=>{
 await page.goto('/catalog.html');
 const knob=page.locator('.catalog-preview').getByRole('slider',{name:'Filter cutoff',exact:true}),box=await knob.boundingBox();
 expect(box).not.toBeNull();
 await page.mouse.move(box.x+box.width/2,box.y+box.height/2);await page.mouse.down();await page.mouse.move(box.x+box.width/2,box.y-40);await page.mouse.up();
 await expect(knob).not.toHaveAttribute('aria-valuenow','0.42');
 const value=await knob.getAttribute('aria-valuenow'),controls=page.getByRole('region',{name:'Story controls'}),control=controls.getByRole('slider',{name:'Value',exact:true});
 expect(Math.abs(Number(await controls.locator('output').textContent())-Number(value))).toBeLessThanOrEqual(.001);
 expect(Math.abs(Number(await control.inputValue())-Number(value))).toBeLessThanOrEqual(.01);
});
