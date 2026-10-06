import {test,expect} from '@playwright/test';
const load=async page=>{await page.goto('/');await page.getByRole('button',{name:/^Patch/}).click();await page.getByRole('button',{name:'Load Felt Kit',exact:true}).click();await expect(page.getByRole('banner').getByRole('status')).toContainText('Ready');};

test('supplied knob drawing supports full-range drag, fine mode, typing and default reset with separate gesture undo',async({page})=>{
  await load(page);await page.getByRole('tab',{name:'Voice',exact:true}).click();
  const knob=page.getByRole('slider',{name:'Cutoff',exact:true}),box=await knob.boundingBox(),x=box.x+box.width/2,y=box.y+box.height/2;
  await page.mouse.move(x,y);await page.mouse.down();await page.mouse.move(x,y-40);await page.mouse.up();
  await expect(knob).toHaveAttribute('aria-valuenow','0.62');
  await page.keyboard.down('Shift');await page.mouse.move(x,y);await page.mouse.down();await page.mouse.move(x,y-40);await page.mouse.up();await page.keyboard.up('Shift');
  await expect(knob).toHaveAttribute('aria-valuenow','0.64');
  await page.keyboard.press('Control+z');await expect(knob).toHaveAttribute('aria-valuenow','0.62');
  await page.keyboard.press('Control+z');await expect(knob).toHaveAttribute('aria-valuenow','0.42');
  await knob.focus();await page.keyboard.press('Home');await page.keyboard.press('PageUp');await expect(knob).toHaveAttribute('aria-valuenow','0.5');
  await knob.dblclick();const input=knob.getByRole('textbox');await expect(input).toBeVisible();await input.fill('2.40 kHz');await input.press('Enter');
  await expect(knob).toHaveAttribute('aria-valuenow','0.42');await knob.dblclick();await input.fill('20 kHz');await input.press('Enter');await expect(knob).toHaveAttribute('aria-valuenow','1');
  await knob.click({modifiers:['Alt']});await expect(knob).toHaveAttribute('aria-valuenow','0.42');
});

test('sample handles and playback edit the accepted region and undo follows navigation',async({page})=>{
  await load(page);const start=page.getByRole('slider',{name:'Region start',exact:true});
  await start.focus();await start.press('PageUp');await expect(start).toHaveAttribute('aria-valuenow','0.02238805337910404');
  await page.getByRole('tab',{name:'Mapping',exact:true}).click();await page.keyboard.press('Control+z');
  await page.getByRole('tab',{name:'Sample',exact:true}).click();await expect(start).toHaveAttribute('aria-valuenow','0.012');
  await page.getByRole('radio',{name:'Sustain loop',exact:true}).click();await expect(page.getByRole('spinbutton',{name:'Loop start',exact:true})).toHaveAttribute('aria-valuenow','2.6510000000000002');
  await expect(page.getByRole('slider',{name:'Loop end',exact:true})).toHaveAttribute('aria-valuenow','0.82');
  await page.keyboard.press('Control+z');await expect(page.getByRole('spinbutton',{name:'Loop start',exact:true})).toHaveCount(0);
});

test('multi-file import offers all mapping policies, shows filename interpretation and undo restores Empty',async({page})=>{
  await page.goto('/');await page.getByLabel('Import sample files').setInputFiles([
    {name:'felt_C4_p.wav',mimeType:'audio/wav',buffer:Buffer.from('mock')},{name:'felt_C4_f.wav',mimeType:'audio/wav',buffer:Buffer.from('mock')},
  ]);
  const mapping=page.getByRole('region',{name:'Import mapping',exact:true});await expect(mapping).toBeVisible();
  for(const policy of ['One per key','By root + velocity','Stack','Round robin'])await expect(mapping.getByRole('button',{name:new RegExp('^'+policy.replace('+','\\+'))})).toBeVisible();
  await expect(page.getByRole('region',{name:'Interpreted mapping'})).toContainText('root 60');
  await mapping.getByRole('button',{name:'Import samples',exact:true}).click();
  await expect(page.getByRole('banner').getByRole('status')).toContainText('Ready');await expect(page.getByRole('button',{name:/^Undo/})).toHaveAttribute('title',/Import samples/);
  await page.keyboard.press('Control+z');await expect(page.getByText('Drop samples here',{exact:true})).toBeVisible();
});
