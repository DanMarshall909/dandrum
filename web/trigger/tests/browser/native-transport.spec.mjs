import {test,expect} from '@playwright/test';
import {session,yaml} from '../native-fixture.mjs';
import {parse,stringify} from 'yaml';

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


test('explicit absent services render accepted controls with unavailable measurements and host-controlled size',async({page},testInfo)=>{
  const errors=[];page.on('pageerror',error=>errors.push(error.message));
  await page.addInitScript(initial=>{
    window.nativeCalls=[];
    window.__DANDRUM__={protocolVersion:1,backendId:'juce-free',capabilities:{metering:false,sourceWaveforms:false,windowTitle:false,editorResize:false},backend:{addEventListener:()=>0,removeEventListener:()=>{},getNativeFunction:name=>async(...args)=>{
      window.nativeCalls.push([name,...args]);if(name==='getSamplerSession')return initial;
      throw Object.assign(new Error('Unsupported '+name),{code:'capabilityUnavailable',method:name});
    }}};
  },(()=>{const graph=parse(yaml);graph.assets.sample_sources[0].regions[0].loop={mode:'forward',start_frame:0,end_frame:10000,crossfade_ms:0};return session(stringify(graph));})());
  await page.goto('/');await expect(page.getByTestId('trigger-editor')).toBeVisible();
  const header=page.getByRole('banner');
  await expect(header.getByTestId('backend-status')).toHaveText('Connected');
  await expect(header).toContainText('Voices — / 8');
  await expect(header.getByText('Meters unavailable',{exact:true})).toBeVisible();
  await expect(header.getByText('Size controlled by host',{exact:true})).toBeVisible();
  await expect(header.getByRole('radio')).toHaveCount(0);
  await expect(header.getByRole('button',{name:'Collapse',exact:true})).toHaveCount(0);
  await expect(page.getByRole('img',{name:'Waveform unavailable'}).first()).toBeVisible();
  await expect(page.locator('[data-knob="drums.level"]')).toBeVisible();
  await expect(page.getByRole('spinbutton',{name:'Loop start',exact:true})).toBeVisible();
  await expect(page.getByRole('img',{name:'Waveform unavailable'})).toHaveCount(4);
  await expect(page.getByRole('img',{name:'Waveform unavailable'}).locator('svg')).toHaveCount(0);
  await page.getByRole('tab',{name:/^Slices/}).click();await expect(page.getByRole('img',{name:'Waveform unavailable'})).toHaveCount(1);
  await page.getByRole('tab',{name:'Routing',exact:true}).click();await expect(page.getByText('Meters unavailable',{exact:true})).toHaveCount(2);
  await expect(page.getByRole('alert')).toHaveCount(0);
  expect(await page.evaluate(()=>window.nativeCalls.map(call=>call[0]))).toEqual(['getSamplerSession']);
  await page.screenshot({path:testInfo.outputPath('native-optional-unavailable.png'),animations:'disabled'});
  expect(errors).toEqual([]);
});

test('accepted voice-limit refresh updates the rendered header while meters remain unavailable',async({page})=>{
  await page.addInitScript(initial=>{
    window.nativeCalls=[];window.nativeSession=initial;window.nativeEvents={};
    window.__DANDRUM__={protocolVersion:1,backendId:'juce-free',capabilities:{metering:false,sourceWaveforms:false,windowTitle:false,editorResize:false},backend:{
      addEventListener:(name,listener)=>{window.nativeEvents[name]=listener;return name;},removeEventListener:name=>{delete window.nativeEvents[name];},
      getNativeFunction:name=>async(...args)=>{window.nativeCalls.push([name,...args]);if(name==='getSamplerSession')return window.nativeSession;throw Error('Unexpected '+name);},
    }};
  },session());
  await page.goto('/');const header=page.getByRole('banner');await expect(header).toContainText('Voices — / 8');
  await page.evaluate(async next=>{
    window.nativeSession=next;await window.nativeEvents.parameterStateChanged({generation:next.document.generation,sequence:0,parameters:[]});
  },session(yaml.replace('max_voices: 8','max_voices: 12'),8));
  await expect(header).toContainText('Voices — / 12');
  await expect(header.getByText('Meters unavailable',{exact:true})).toBeVisible();
  expect(await page.evaluate(()=>window.nativeCalls.map(call=>call[0]))).toEqual(['getSamplerSession','getSamplerSession']);
  await expect(page.getByRole('alert')).toHaveCount(0);
});
