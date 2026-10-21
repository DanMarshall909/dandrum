// Test augmentation of a copied app.js, never installed in the production bundle.
// Signal success only through existing authenticated note commands and real PCM.
(async () => {
  const require = (condition, message) => { if (!condition) throw new Error(`G1: ${message}`); };
  try {
    for (let attempt = 0; attempt < 400; attempt++) {
      if (document.querySelector('[data-testid="trigger-editor"]')) break;
      require(!document.querySelector('[role="alert"]'), 'native startup failed');
      await new Promise(resolve => setTimeout(resolve, 25));
    }
    const descriptor = window.__DANDRUM__;
    require(descriptor?.backendId === 'juce-free' && descriptor.protocolVersion === 1, 'native transport');
    require(window.__DANDRUM_NATIVE__.instanceId === window.__G1_EXPECT_INSTANCE__, 'distinct factory instance wire identity');
    const frame = document.querySelector('[data-testid="trigger-editor"]');
    const badge = document.querySelector('[data-testid="backend-status"]');
    require(frame && document.querySelector('.preview--native'), 'production React native shell mounted');
    require(badge?.dataset.backend === 'native' && badge.textContent === 'Connected', 'connected badge');
    require(!document.querySelector('[role="alert"]'), 'no error shell');
    require(frame.querySelector('header')?.textContent.includes('Advanced Drum Kit'), 'accepted kit rendered');
    require(document.body.textContent.includes('Meters unavailable') && document.body.textContent.includes('Size controlled by host'), 'honest optional services');
    require(!document.body.textContent.includes('Connecting to native sampler…'), 'startup promise consumed');
    const invoke = (method, ...args) => descriptor.backend.getNativeFunction(method)(...args);
    const session = await invoke('getSamplerSession');
    require(session.connection.instanceId === window.__DANDRUM_NATIVE__.instanceId
      && session.connection.attachmentId === window.__DANDRUM_NATIVE__.attachmentId, 'live native session identity');
    require(session.document.parameters.length === 23 && session.document.maps[0].zones.some(zone => zone.keyLow === 36), 'accepted native document');
    require(session.document.sources[0].sampleRateHz === 48000 && session.document.sources[0].frameCount === '51000', 'real sample metadata');
    // Required unsupported commands reject; a successful placeholder is a defect.
    let unavailable = false;
    try { await invoke('requestSourceWaveform', 'drums', 0, 8, session.document.generation); }
    catch (error) { unavailable = error.code === 'capabilityUnavailable'; }
    require(unavailable, 'unsupported native operation rejects');
    // A completed authenticated note roundtrip precedes the audible readiness note.
    await invoke('noteOn', 60, 1, session.document.generation);
    await invoke('noteOff', 60, session.document.generation);
    await invoke('noteOn', 36, 1, session.document.generation);
    await invoke('noteOff', 36, session.document.generation);
  } catch (error) { console.error(error); }
})();
