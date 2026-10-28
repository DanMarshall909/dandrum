import {test,expect} from '@playwright/test';
import {session,yaml,editorProfile} from '../native-fixture.mjs';
import {parse,stringify} from 'yaml';

test.beforeEach(async({page})=>{await page.addInitScript(profile=>{window.__TEST_EDITOR_PROFILE__=profile;},editorProfile());});

test('the production entrypoint uses the versioned host instead of the comparison bridge',async({page})=>{
  await page.addInitScript(initial=>{
    window.nativeCalls=[];
    const backend={addEventListener:()=>0,removeEventListener:()=>{},getNativeFunction:name=>async(...args)=>{
      window.nativeCalls.push([name,...args]);
      if(name==='negotiateEditorProtocol')return window.__TEST_EDITOR_PROFILE__;
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
    window.__DANDRUM__={protocolVersion:1,backendId:'juce-free',backend:{addEventListener:()=>0,removeEventListener:()=>{},getNativeFunction:name=>async()=>{
      if(name==='negotiateEditorProtocol')return window.__TEST_EDITOR_PROFILE__;
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
      window.nativeCalls.push([name,...args]);if(name==='negotiateEditorProtocol')return window.__TEST_EDITOR_PROFILE__;if(name==='getSamplerSession')return initial;
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
  expect(await page.evaluate(()=>window.nativeCalls.map(call=>call[0]))).toEqual(['negotiateEditorProtocol','getSamplerSession']);
  await page.screenshot({path:testInfo.outputPath('native-optional-unavailable.png'),animations:'disabled'});
  expect(errors).toEqual([]);
});

test('accepted voice-limit refresh updates the rendered header while meters remain unavailable',async({page})=>{
  await page.addInitScript(initial=>{
    window.nativeCalls=[];window.nativeSession=initial;window.nativeEvents={};
    window.__DANDRUM__={protocolVersion:1,backendId:'juce-free',capabilities:{metering:false,sourceWaveforms:false,windowTitle:false,editorResize:false},backend:{
      addEventListener:(name,listener)=>{window.nativeEvents[name]=listener;return name;},removeEventListener:name=>{delete window.nativeEvents[name];},
      getNativeFunction:name=>async(...args)=>{window.nativeCalls.push([name,...args]);if(name==='negotiateEditorProtocol')return window.__TEST_EDITOR_PROFILE__;if(name==='getSamplerSession')return window.nativeSession;throw Error('Unexpected '+name);},
    }};
  },session());
  await page.goto('/');const header=page.getByRole('banner');await expect(header).toContainText('Voices — / 8');
  await page.evaluate(async next=>{
    window.nativeSession=next;await window.nativeEvents.parameterStateChanged({generation:next.document.generation,sequence:0,parameters:[]});
  },session(yaml.replace('max_voices: 8','max_voices: 12'),8));
  await expect(header).toContainText('Voices — / 12');
  await expect(header.getByText('Meters unavailable',{exact:true})).toBeVisible();
  expect(await page.evaluate(()=>window.nativeCalls.map(call=>call[0]))).toEqual(['negotiateEditorProtocol','getSamplerSession','getSamplerSession']);
  await expect(page.getByRole('alert')).toHaveCount(0);
});


test('trusted Wry bootstrap connects the production entrypoint through correlated native messages',async({page})=>{
  const errors=[];page.on('pageerror',error=>errors.push(error.message));
  await page.addInitScript(initial=>{
    window.nativeCalls=[];
    window.__DANDRUM_NATIVE__={protocolVersion:1,instanceId:initial.connection.instanceId,attachmentId:initial.connection.attachmentId,nextRequestId:'41'};
    window.ipc={postMessage(message){
      if(this!==window.ipc)throw Error('Wry IPC receiver was lost');
      const request=JSON.parse(message);window.nativeCalls.push(request);
      if(window.nativeCalls.length>2)throw Error('Unexpected repeated native request');
      if(!['negotiateEditorProtocol','getSamplerSession'].includes(request.method))throw Error('Unexpected native method '+request.method);
      window.__DANDRUM__.receiveNativeMessage(JSON.stringify({version:request.version,instanceId:initial.connection.instanceId,attachmentId:initial.connection.attachmentId,requestId:request.requestId,status:'accepted',result:request.method==='negotiateEditorProtocol'?window.__TEST_EDITOR_PROFILE__:initial}));
    }};
  },session());
  await page.goto('/');
  await expect(page.getByRole('banner').locator('[data-backend="native"]')).toBeVisible();
  await expect(page.locator('[data-knob="drums.level"]')).toBeVisible();
  await expect(page.getByRole('banner').getByTestId('backend-status')).toHaveText('Connected');
  expect(await page.evaluate(()=>window.nativeCalls)).toEqual([{version:1,attachmentId:'9007199254740993',requestId:'41',method:'negotiateEditorProtocol',args:[2]},{version:2,attachmentId:'9007199254740993',requestId:'42',method:'getSamplerSession',args:[]}]);
  await expect(page.getByRole('alert')).toHaveCount(0);
  expect(errors).toEqual([]);
});

test('page disposal closes a pending native initialization once without rendering a late failure',async({page})=>{
  const errors=[];page.on('pageerror',error=>errors.push(error.message));
  await page.addInitScript(()=>{
    window.nativeCalls=[];
    window.__DANDRUM_NATIVE__={protocolVersion:1,instanceId:'17',attachmentId:'23'};
    window.ipc={postMessage(message){window.nativeCalls.push(JSON.parse(message));if(window.nativeCalls.length>1)throw Error('Unexpected native request after disposal');}};
  });
  await page.goto('/');await expect(page.getByRole('status')).toHaveText('Connecting to native sampler…');
  await page.evaluate(async()=>{
    const descriptor=window.__DANDRUM__;
    const listener=()=>{throw Error('Native event delivered after page disposal');};
    descriptor.backend.addEventListener('instrumentSessionChanged',listener);
    window.dispatchEvent(new Event('pagehide'));window.dispatchEvent(new Event('pagehide'));
    window.lateDelivery=descriptor.receiveNativeMessage(JSON.stringify({version:1,instanceId:'17',attachmentId:'23',event:'instrumentSessionChanged',sequence:'1',payload:{generation:9}}));
    window.afterClose=await descriptor.backend.getNativeFunction('getSamplerSession')().catch(error=>error.code);
  });
  await expect(page.getByRole('alert')).toHaveCount(0);
  await expect(page.getByTestId('trigger-editor')).toHaveCount(0);
  expect(await page.evaluate(()=>({calls:window.nativeCalls,late:window.lateDelivery,closed:window.afterClose}))).toEqual({calls:[{version:1,attachmentId:'23',requestId:'1',method:'negotiateEditorProtocol',args:[2]}],late:'ignored',closed:'closed'});
  expect(errors).toEqual([]);
});

test('failed bootstrap initialization renders its native error and closes pending browser work',async({page})=>{
  await page.addInitScript(()=>{
    window.nativeCalls=[];
    window.__DANDRUM_NATIVE__={protocolVersion:1,instanceId:'17',attachmentId:'23'};
    window.ipc={postMessage(message){
      const request=JSON.parse(message);window.nativeCalls.push(request);
      if(window.nativeCalls.length>3)throw Error('Unexpected native request after failure');
      if(request.method==='negotiateEditorProtocol'){
        window.__DANDRUM__.receiveNativeMessage(JSON.stringify({version:1,instanceId:'17',attachmentId:'23',requestId:request.requestId,status:'accepted',result:window.__TEST_EDITOR_PROFILE__}));return;
      }
      if(request.method==='getSamplerSession'){
        window.pendingOutcome=window.__DANDRUM__.backend.getNativeFunction('pending')().catch(error=>error.code);
        window.__DANDRUM__.receiveNativeMessage(JSON.stringify({version:request.version,instanceId:'17',attachmentId:'23',requestId:request.requestId,status:'rejected',error:{code:'capabilityUnavailable',message:'Session unavailable'}}));
      }
    }};
  });
  await page.goto('/');await expect(page.getByRole('alert')).toHaveText('Native sampler failed: Session unavailable');
  expect(await page.evaluate(()=>window.pendingOutcome)).toBe('closed');
  expect(await page.evaluate(()=>window.nativeCalls.map(call=>call.method))).toEqual(['negotiateEditorProtocol','getSamplerSession','pending']);
  await expect(page.getByTestId('trigger-editor')).toHaveCount(0);
});

test('invalid trusted bootstrap is visible before any native call or preview',async({page})=>{
  await page.addInitScript(()=>{window.nativeCalls=[];window.__DANDRUM_NATIVE__={protocolVersion:2};window.ipc={postMessage:message=>window.nativeCalls.push(message)};});
  await page.goto('/');await expect(page.getByRole('alert')).toHaveText('Native sampler failed: invalidNativeBootstrap');
  await expect(page.getByTestId('trigger-editor')).toHaveCount(0);
  expect(await page.evaluate(()=>window.nativeCalls)).toEqual([]);
});

test('page disposal sends held-note cleanup before closing the local broker',async({page})=>{
  const errors=[];page.on('pageerror',error=>errors.push(error.message));
  await page.addInitScript(initial=>{
    window.nativeCalls=[];
    window.__DANDRUM_NATIVE__={protocolVersion:1,instanceId:initial.connection.instanceId,attachmentId:initial.connection.attachmentId};
    window.ipc={postMessage(message){
      const request=JSON.parse(message);window.nativeCalls.push(request);
      if(window.nativeCalls.length>5)throw Error('Repeated native cleanup');
      if(!['negotiateEditorProtocol','getSamplerSession','noteOn','noteOff','noteHeartbeat'].includes(request.method))throw Error('Unexpected '+request.method);
      window.__DANDRUM__.receiveNativeMessage(JSON.stringify({version:request.version,instanceId:initial.connection.instanceId,attachmentId:initial.connection.attachmentId,requestId:request.requestId,status:'accepted',result:request.method==='negotiateEditorProtocol'?window.__TEST_EDITOR_PROFILE__:request.method==='getSamplerSession'?initial:true}));
    }};
  },session());
  await page.goto('/');await expect(page.getByTestId('trigger-editor')).toBeVisible();
  await page.keyboard.down('a');
  await expect.poll(()=>page.evaluate(()=>window.nativeCalls.filter(call=>call.method==='noteOn').length)).toBe(1);
  await page.evaluate(()=>{window.dispatchEvent(new Event('pagehide'));window.dispatchEvent(new Event('pagehide'));});
  const notes=await page.evaluate(()=>window.nativeCalls.filter(call=>['noteOn','noteOff'].includes(call.method)));
  expect(notes).toHaveLength(2);expect(notes[0].method).toBe('noteOn');expect(notes[1].method).toBe('noteOff');
  expect(notes[0].args).toEqual([48,100/127,7]);expect(notes[1].args).toEqual([48,7]);
  expect(await page.evaluate(()=>window.__DANDRUM__.backend.getNativeFunction('getSamplerSession')().catch(error=>error.code))).toBe('closed');
  expect(errors).toEqual([]);
});

test('a code-only native protocol rejection displays its explicit capability failure',async({page})=>{
  await page.addInitScript(()=>{
    window.nativeCalls=[];
    window.__DANDRUM_NATIVE__={protocolVersion:1,instanceId:'17',attachmentId:'23'};
    window.ipc={postMessage(message){
      const request=JSON.parse(message);window.nativeCalls.push(request);
      if(window.nativeCalls.length>1)throw Error('Unexpected native retry');
      window.__DANDRUM__.receiveNativeMessage(JSON.stringify({version:1,instanceId:'17',attachmentId:'23',requestId:request.requestId,status:'rejected',error:{code:'capabilityUnavailable'}}));
    }};
  });
  await page.goto('/');await expect(page.getByRole('alert')).toHaveText('Native sampler failed: capabilityUnavailable');
  await expect(page.getByTestId('trigger-editor')).toHaveCount(0);
  expect(await page.evaluate(()=>window.nativeCalls)).toEqual([{version:1,attachmentId:'23',requestId:'1',method:'negotiateEditorProtocol',args:[2]}]);
  expect(await page.evaluate(()=>window.__DANDRUM__.backend.getNativeFunction('getSamplerSession')().catch(error=>error.code))).toBe('closed');
});


for(const input of ['DOM event','browser keyboard'])test(`production native mapping ${input} Escape releases its choice`,async({page})=>{
  const errors=[];page.on('pageerror',error=>errors.push(error.message));
  await page.addInitScript(initial=>{
    window.nativeCalls=[];let phase='ready';
    const hash='2e484972f38876c961b3c23fb4952487dce3e18501c7fb0efd1b8cb7011734d5';
    const file=name=>({name,resourceId:`selected/${hash}.wav`,contentSha256:hash,encodedBytes:60,sampleRateHz:32000,channelCount:2,frameCount:4});
    window.__DANDRUM__={protocolVersion:1,backendId:'juce-free',capabilities:{metering:false,sourceWaveforms:false,windowTitle:false,editorResize:false},backend:{
      addEventListener:()=>0,removeEventListener:()=>{},getNativeFunction:name=>async(...args)=>{
        window.nativeCalls.push([name,...args]);if(window.nativeCalls.length>30)throw Error('Unexpected repeated request');
        if(name==='negotiateEditorProtocol')return window.__TEST_EDITOR_PROFILE__;
        if(name==='getSamplerSession')return initial;
        if(name==='chooseSamplerFiles')return {selectionId:'1',phase:'choosing'};
        if(name==='getSamplerSelectionStatus')return {selectionId:'1',phase,files:[file('first.wav'),file('second.wav')]};
        if(name==='releaseSamplerSelection'){phase='released';return {selectionId:'1',phase};}
        throw Error('Unexpected '+name);
      },
    }};
  },session());
  await page.goto('/');await expect(page.getByTestId('trigger-editor')).toBeVisible();
  if(!await page.getByRole('button',{name:'Add source…',exact:true}).isVisible())await page.getByRole('button',{name:'Instrument tree',exact:true}).click();
  await page.getByRole('button',{name:'Add source…',exact:true}).click();
  await expect(page.getByRole('region',{name:'Import mapping',exact:true})).toBeVisible();
  if(input==='DOM event')await page.evaluate(()=>window.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true})));
  else await page.keyboard.press('Escape');
  await expect(page.getByRole('region',{name:'Import mapping',exact:true})).toHaveCount(0);
  expect(await page.evaluate(()=>window.nativeCalls.filter(call=>call[0]==='releaseSamplerSelection'))).toEqual([['releaseSamplerSelection','1']]);
  expect(errors).toEqual([]);
});
