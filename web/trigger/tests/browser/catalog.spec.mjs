import {test,expect} from './fixtures.mjs';
test('modulator editor renders its descriptor-backed controls in isolation',async({page})=>{
 const errors=[];page.on('pageerror',error=>errors.push(error.message));await page.goto('/catalog.html');await page.locator('.catalog-nav').getByRole('button',{name:'ModulatorEditor',exact:true}).click();await expect(page.locator('.catalog-preview [data-story-error]')).toHaveCount(0);expect(errors).toEqual([]);
});
test('every public component renders independently in all nine catalog states, with editable props and width/background controls',async({page})=>{
 test.setTimeout(120000);const errors=[];page.on('pageerror',e=>errors.push(e.message));await page.goto('/catalog.html');
 const nav=page.locator('.catalog-nav');await expect(nav.getByRole('button').first()).toBeVisible();const names=await nav.getByRole('button').allTextContents();expect(names.length).toBeGreaterThan(90);
 for(const name of names){await nav.getByRole('button',{name,exact:true}).click({force:true});for(const state of ['default','hover','selected','disabled','empty','loading','error','long text','minimum width']){
  await page.getByRole('region',{name:'Story controls'}).locator('select').first().selectOption(state,{force:true});await expect(page.locator('.catalog-preview')).not.toContainText('name.trim is not a function');await expect(page.locator('.catalog-preview [data-story-error]')).toHaveCount(0);
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
test('clicking the catalog knob value opens its editor without beginning a dial gesture',async({page})=>{
 await page.goto('/catalog.html');
 const knob=page.locator('.catalog-preview').getByRole('slider',{name:'Filter cutoff',exact:true});
 await knob.hover();const before=await knob.getAttribute('aria-valuenow');
 const value=knob.getByTitle('Click to type a value');await expect(value).toBeVisible();await value.click();
 await expect(knob.getByRole('textbox')).toBeVisible();await expect(knob).toHaveAttribute('aria-valuenow',before);
});
test('catalog knob retains a typed fraction and drags by its denominator',async({page})=>{
 await page.goto('/catalog.html');const knob=page.locator('.catalog-preview').getByRole('slider',{name:'Filter cutoff',exact:true});
 await knob.hover();await expect(knob.getByRole('status',{name:'Effective value'})).toBeVisible();await knob.getByTitle('Click to type a value').click();let input=knob.getByRole('textbox');await input.fill('1/3');await input.press('Enter');
 await expect(knob).toHaveAttribute('aria-valuenow',String(1/3));await expect(knob.getByTitle('Click to type a value')).toContainText('33%');
 await knob.getByTitle('Click to type a value').click();input=knob.getByRole('textbox');await expect(input).toHaveValue('1/3');await input.press('Escape');
 const box=await knob.boundingBox(),x=box.x+box.width/2,y=box.y+box.height/2;await page.mouse.move(x,y);await page.mouse.down();await page.mouse.move(x,y-6);await page.mouse.up();
 await expect(knob).toHaveAttribute('aria-valuenow',String(2/3));
});
test('an external authoritative update clears retained fraction edit state',async({page})=>{
 await page.goto('/catalog.html');const knob=page.locator('.catalog-preview').getByRole('slider',{name:'Filter cutoff',exact:true}),controls=page.getByRole('region',{name:'Story controls'});
 await knob.hover();await knob.getByTitle('Click to type a value').click();await knob.getByRole('textbox').fill('1/3');await knob.getByRole('textbox').press('Enter');
 const control=controls.getByRole('slider',{name:'Value',exact:true});await control.focus();await control.press('End');await knob.getByTitle('Click to type a value').click();await expect(knob.getByRole('textbox')).toHaveValue('100%');
});
test('catalog parameter variants expose their explicit display and accessible text',async({page})=>{
 await page.goto('/catalog.html');const controls=page.getByRole('region',{name:'Story controls'}),variant=controls.locator('label').filter({hasText:/^Variant/}).locator('select'),knob=page.locator('.catalog-preview').getByRole('slider',{name:'Filter cutoff',exact:true});
 for(const [name,visual,spoken] of [['percent','42%','42 percent'],['unit-interval','.42','0.42'],['bipolar-percent','−16%','16 percent negative'],['bipolar-unit','−.16','0.16 negative'],['pan','L 16%','16 percent left']]){
  await variant.selectOption(name,{force:true});await knob.hover();await expect(knob.getByTitle('Click to type a value')).toContainText(visual);await expect(knob).toHaveAttribute('aria-valuetext',spoken);await page.mouse.move(700,80);
 }
 await variant.selectOption('frequency',{force:true});await expect(knob).toHaveAttribute('aria-valuemin','20');await expect(knob).toHaveAttribute('aria-valuemax','20000');
});
test('catalog scale-mark presets change the maintained knob drawing',async({page})=>{
 await page.goto('/catalog.html');const controls=page.getByRole('region',{name:'Story controls'}),preset=controls.locator('label').filter({hasText:/^Scale marks/}).locator('select'),knob=page.locator('.catalog-preview').getByRole('slider',{name:'Filter cutoff',exact:true});
 await preset.selectOption('none');const baseline=await knob.locator('svg line').count();await preset.selectOption('dense');await expect(knob.locator('svg line')).toHaveCount(baseline+9);
});
test('descriptor validation preserves the value and clamps valid out-of-range entry',async({page})=>{
 await page.goto('/catalog.html');const knob=page.locator('.catalog-preview').getByRole('slider',{name:'Filter cutoff',exact:true});await knob.hover();await knob.getByTitle('Click to type a value').click();const input=knob.getByRole('textbox'),before=await knob.getAttribute('aria-valuenow');
 await input.fill('not a value');await input.press('Enter');await expect(input).toBeVisible();await expect(knob.getByRole('alert')).toContainText('valid');await expect(knob).toHaveAttribute('aria-valuenow',before);await input.press('Escape');
 await knob.getByTitle('Click to type a value').click();await knob.getByRole('textbox').fill('90%');await knob.getByRole('textbox').press('Escape');await expect(knob).toHaveAttribute('aria-valuenow',before);
 await knob.getByTitle('Click to type a value').click();await knob.getByRole('textbox').fill('200%');await knob.getByRole('textbox').press('Enter');await expect(knob).toHaveAttribute('aria-valuenow','1');
});
test('numeric field uses the same descriptor and retained fraction edit state',async({page})=>{
 await page.goto('/catalog.html');await page.locator('.catalog-nav').getByRole('button',{name:'NumericField',exact:true}).click();const field=page.locator('.catalog-preview').getByRole('spinbutton',{name:'Filter cutoff',exact:true});
 await field.click();let input=field.getByRole('textbox');await input.fill('1/3');await input.press('Enter');await expect(field).toHaveAttribute('aria-valuenow',String(1/3));
 await field.click();input=field.getByRole('textbox');await expect(input).toHaveValue('1/3');await input.press('Escape');
});
