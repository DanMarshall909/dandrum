import {test,expect} from '@playwright/test';

test('the supplied workspaces and frame controls remain reachable',async({page})=>{
  const errors=[];page.on('pageerror',error=>errors.push(error.message));
  await page.goto('/');
  await page.getByRole('button',{name:/^Patch/}).click();
  await page.getByRole('button',{name:'Load Felt Kit',exact:true}).click();
  await expect(page.getByRole('banner').getByRole('status')).toContainText('Ready');
  await expect(page.getByTestId('trigger-editor')).toHaveCSS('width','1200px');
  const workspaces=[
    ['Sample','slider','Start'],['Slices 8','button','Detect transients'],
    ['Mapping','button','Map by root'],['Layers','button','Round robin'],
    ['Voice','slider','Cutoff'],['Modulation 14','slider','Cutoff'],
    ['Routing','button','Show graph'],['Effects',null,null],
  ];
  for(const [name,role,control] of workspaces){
    const tab=page.getByRole('tab',{name,exact:true});await tab.click();
    await expect(tab).toHaveAttribute('aria-selected','true');
    await expect(page.getByRole('main')).toBeVisible();
    if(control)await expect(page.getByRole(role,{name:control,exact:true}).first()).toBeVisible();
  }
  await page.getByRole('button',{name:'Collapse',exact:true}).click();
  await expect(page.getByRole('main')).not.toBeVisible();
  await expect(page.getByRole('slider',{name:'Tone',exact:true})).toBeVisible();
  await page.getByRole('button',{name:'Open editor',exact:true}).click();
  await page.getByRole('radio',{name:'Min',exact:true}).click();
  await expect(page.getByRole('button',{name:'Instrument tree',exact:true})).toBeVisible();
  await page.getByRole('radio',{name:'Expanded',exact:true}).click();
  await expect(page.getByRole('main')).toContainText('Key map · read-only overview');
  await expect(page.getByRole('button',{name:'Add group',exact:true})).toBeVisible();
  expect(errors).toEqual([]);
});
