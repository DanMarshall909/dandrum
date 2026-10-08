import {test,expect} from '@playwright/test';
const load=async page=>{await page.goto('/');await page.getByRole('button',{name:/^Patch/}).click();await page.getByRole('button',{name:'Load Felt Kit',exact:true}).click();await expect(page.getByRole('banner').getByRole('status')).toContainText('Ready');};
test('selector policies, crossfade and candidate mute/solo are accepted edits and undo updates their controls',async({page})=>{
  await load(page);await page.getByRole('tab',{name:'Layers',exact:true}).click();const width=page.getByRole('spinbutton',{name:'Width',exact:true});
  await expect(width).toHaveAttribute('aria-valuenow','12');await width.press('ArrowUp');await expect(width).toHaveAttribute('aria-valuenow','13');
  await page.keyboard.press('Tab');await page.keyboard.press('Control+z');await expect(width).toHaveAttribute('aria-valuenow','12');
  await page.locator('[data-node="snare-hard"]').click();await expect(page.locator('[data-ctx^="candidate|"]')).toHaveCount(3);
  const mute=page.getByRole('button',{name:'Mute Hard B',exact:true});await mute.click();await expect(mute).toHaveAttribute('aria-pressed','true');
  await page.keyboard.press('Control+z');await expect(mute).toHaveAttribute('aria-pressed','false');
  await page.getByRole('button',{name:'Solo Hard C',exact:true}).click();await expect(page.getByRole('button',{name:'Solo Hard C',exact:true})).toHaveAttribute('aria-pressed','true');
  await page.getByRole('button',{name:'Weighted',exact:true}).click();await expect(page.getByRole('button',{name:/^Undo/})).toHaveAttribute('title',/Change selector mode/);
  await page.keyboard.press('Control+z');await expect(page.getByRole('button',{name:'Round robin',exact:true})).toHaveAttribute('aria-pressed','true');
});
test('voice module modes, voice policy, envelope values and template order use the model',async({page})=>{
  await load(page);await page.getByRole('tab',{name:'Voice',exact:true}).click();await page.getByRole('radio',{name:'HP',exact:true}).click();
  await page.keyboard.press('Control+z');await expect(page.getByRole('radio',{name:'LP',exact:true})).toHaveAttribute('aria-checked','true');
  await page.getByRole('radio',{name:'Mono',exact:true}).click();await expect(page.getByRole('radio',{name:'Mono',exact:true})).toHaveAttribute('aria-checked','true');
  await page.keyboard.press('Control+z');await expect(page.getByRole('radio',{name:'Poly',exact:true})).toHaveAttribute('aria-checked','true');
  const sustain=page.getByRole('slider',{name:'Amp envelope sustain',exact:true});await sustain.press('PageUp');await expect(sustain).toHaveAttribute('aria-valuenow','0.51');
  await page.keyboard.press('Control+z');await expect(sustain).toHaveAttribute('aria-valuenow','0.5');
  await page.getByRole('button',{name:'Edit template…',exact:true}).click();await page.getByRole('button',{name:'Move Filter up',exact:true}).click();await page.getByRole('button',{name:'Done',exact:true}).click();
  await expect(page.getByRole('button',{name:/^Undo/})).toHaveAttribute('title',/Move voice module/);
});
test('routing inheritance, sends, graph, processor add and bypass are editable through the reused widgets',async({page})=>{
  await load(page);await page.getByRole('tab',{name:'Routing',exact:true}).click();await page.getByRole('button',{name:'Show graph',exact:true}).click();await expect(page.getByRole('img',{name:'Routing graph'})).toContainText('Snare');
  const output=page.getByRole('button',{name:/Output for Snare/});await output.click();await page.getByRole('menuitem',{name:'Drums 3/4',exact:true}).click();
  await expect(output).toContainText('Drums');await page.keyboard.press('Control+z');await expect(output).toContainText('Snare 5/6');
  const send=page.getByRole('slider',{name:'Snare Reverb send',exact:true});await send.press('ArrowUp');await expect(send).toHaveAttribute('aria-valuenow','0.35');
  await page.keyboard.press('Control+z');await expect(send).toHaveAttribute('aria-valuenow','0.3');
  await page.getByRole('tab',{name:'Effects',exact:true}).click();await page.getByRole('button',{name:'Add module to Drums',exact:true}).click();
  await page.getByRole('dialog',{name:'Add processor',exact:true}).getByRole('button',{name:'Delay',exact:true}).click();await expect(page.locator('[data-chain="drums"]').getByRole('button',{name:'Delay',exact:true})).toBeVisible();
  await page.keyboard.press('Control+z');await expect(page.locator('[data-chain="drums"]').getByRole('button',{name:'Delay',exact:true})).toHaveCount(0);
  const bypass=page.locator('[data-chain="drums"]').getByRole('button',{name:'Bypass module',exact:true}).first();await bypass.click();await expect(page.locator('[data-chain="drums"]').getByRole('button',{name:'Enable module',exact:true})).toBeVisible();
});
