// Run normally, then with SLOW_MO=700 --headed; the fixture exercises geometry only.
import {test,expect} from '@playwright/test';
import {session} from '../native-fixture.mjs';

test.use({launchOptions:{executablePath:process.env.TRIGGER_CHROME??'/usr/bin/google-chrome',slowMo:Number(process.env.SLOW_MO??0)}});
test('native backend status stays in the header and the editor fits actual host viewports',async({page},testInfo)=>{
  const errors=[];page.on('pageerror',error=>errors.push(error.message));
  await page.addInitScript(initial=>{
    window.nativeResizeRequests=[];
    window.__JUCE__={backend:{addEventListener:()=>0,removeEventListener:()=>{},getNativeFunction:name=>async(...args)=>{
      if(name==='getSamplerSession')return initial;
      if(name==='subscribeMeter'||name==='setMeterVisible')return true;
      if(name==='getMeterPacket')return null;
      if(name==='requestSourceWaveform')return {status:'accepted',job_id:'1',generation:initial.document.generation};
      if(name==='getWaveformJobStatus')return {state:'ready',generation:initial.document.generation,result:{buckets:[]}};
      if(name==='prepareSampler')return 'Layout test preparation failed';
      if(name==='resizeSamplerEditor'){window.nativeResizeRequests.push(args);return {status:'accepted',width:820,height:560};}
      return null;
    }}};
  },session());
  await page.setViewportSize({width:1180,height:860});await page.goto('/');
  const editor=page.getByTestId('trigger-editor'),header=page.getByRole('banner');
  await expect(editor).toBeVisible();
  await page.screenshot({path:testInfo.outputPath('native-1180.png'),animations:'disabled'});
  await expect(header.locator('[data-backend="native"]')).toBeVisible();
  await expect(page.locator('.preview > [data-backend]')).toHaveCount(0);
  async function fits(){
    const geometry=await editor.evaluate(element=>{const rect=element.getBoundingClientRect();return {left:rect.left,top:rect.top,right:rect.right,bottom:rect.bottom,width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth>innerWidth,font:Math.min(...Array.from(element.querySelectorAll('button')).map(button=>Number.parseFloat(getComputedStyle(button).fontSize)))};});
    expect(geometry.left).toBeGreaterThanOrEqual(0);expect(geometry.top).toBeGreaterThanOrEqual(0);expect(geometry.right).toBeLessThanOrEqual(geometry.width);expect(geometry.bottom).toBeLessThanOrEqual(geometry.height);expect(geometry.overflow).toBe(false);expect(geometry.font).toBeGreaterThanOrEqual(11);
    await expect(header.getByRole('button',{name:/^Patch/})).toBeInViewport();await expect(header.getByRole('button',{name:'Collapse',exact:true})).toBeInViewport();
  }
  await fits();
  for(const viewport of [{width:820,height:560},{width:1500,height:1000}]){
    await page.setViewportSize(viewport);
    for(const [mode,id] of [['Min','min'],['Default','default'],['Expanded','exp']]){await header.getByRole('radio',{name:mode,exact:true}).click();await fits();expect(await page.evaluate(()=>window.nativeResizeRequests.at(-1))).toEqual([id]);}
    await page.screenshot({path:testInfo.outputPath(`native-${viewport.width}.png`),animations:'disabled'});
    await header.getByRole('button',{name:'Collapse',exact:true}).click();await expect(header.getByRole('button',{name:'Open editor',exact:true})).toBeInViewport();expect(await page.evaluate(()=>window.nativeResizeRequests.at(-1))).toEqual(['perf']);await header.getByRole('button',{name:'Open editor',exact:true}).click();expect(await page.evaluate(()=>window.nativeResizeRequests.at(-1))).toEqual(['exp']);
  }
  await header.getByRole('button',{name:/^Patch/}).click();await page.getByRole('button',{name:'New empty patch',exact:true}).click();
  await expect(header.getByRole('status')).toContainText('Operation failed');await expect(header.getByRole('status')).toContainText('Layout test preparation failed');
  expect(errors).toEqual([]);
});
