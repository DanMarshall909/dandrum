import {test,expect} from '@playwright/test';
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
