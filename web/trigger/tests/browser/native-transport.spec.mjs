import {test,expect} from '@playwright/test';
import {session} from '../native-fixture.mjs';

test('the production entrypoint uses the versioned host instead of the comparison bridge',async({page})=>{
  await page.addInitScript(initial=>{
    window.nativeCalls=[];
    const backend={addEventListener:()=>0,removeEventListener:()=>{},getNativeFunction:name=>async(...args)=>{
      window.nativeCalls.push([name,...args]);
      if(name==='getSamplerSession')return initial;
      if(name==='subscribeMeter'||name==='setMeterVisible')return true;
      if(name==='requestSourceWaveform')return {status:'accepted',job_id:'1',generation:initial.document.generation};
      if(name==='getWaveformJobStatus')return {state:'ready',generation:initial.document.generation,result:{buckets:[]}};
      return null;
    }};
    window.__DANDRUM__={protocolVersion:1,backendId:'juce-free',backend};
    window.__JUCE__={backend:{getNativeFunction:()=>{throw new Error('Comparison bridge must remain unused');}}};
  },session());
  await page.goto('/');await expect(page.getByTestId('trigger-editor')).toBeVisible();
  await expect(page.getByRole('banner').locator('[data-backend="native"]')).toBeVisible();
  expect(await page.evaluate(()=>window.nativeCalls.filter(call=>call[0]==='setTriggerWindowTitle'))).toEqual([['setTriggerWindowTitle','Advanced Drum Kit']]);
  await expect(page.getByRole('alert')).toHaveCount(0);
});

test('an incompatible declared native host displays an error and never opens the silent preview',async({page})=>{
  await page.addInitScript(()=>{window.__DANDRUM__={protocolVersion:2};});
  await page.goto('/');
  await expect(page.getByRole('alert')).toHaveText('Native sampler failed: Unsupported native transport protocol version');
  await expect(page.getByTestId('trigger-editor')).toHaveCount(0);
});

test('a rejected native snapshot displays its capability error without simulating an instrument',async({page})=>{
  await page.addInitScript(()=>{
    window.__DANDRUM__={protocolVersion:1,backendId:'juce-free',backend:{addEventListener:()=>0,removeEventListener:()=>{},getNativeFunction:()=>async()=>{
      throw Object.assign(new Error('Session snapshot capability unavailable'),{code:'capabilityUnavailable',method:'getSamplerSession'});
    }}};
  });
  await page.goto('/');await expect(page.getByRole('alert')).toHaveText('Native sampler failed: Session snapshot capability unavailable');
  await expect(page.getByTestId('trigger-editor')).toHaveCount(0);
});

test('an ordinary browser continues to show an explicitly silent preview',async({page})=>{
  await page.goto('/');await expect(page.getByTestId('trigger-editor')).toBeVisible();
  await expect(page.getByRole('banner').locator('[data-backend="mock"]')).toContainText(/silent/i);
  await expect(page.getByRole('alert')).toHaveCount(0);
});
