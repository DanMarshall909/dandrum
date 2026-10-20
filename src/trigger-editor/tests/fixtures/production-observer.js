// Native test inspection only: the production package and transport are unchanged.
// This observer does not qualify CSP script execution or native-session behavior.
(async () => {
  const report = value => window.ipc.postMessage(value);
  const require = (condition, message) => { if (!condition) throw new Error(message); };
  try {
    let frame;
    for (let attempt = 0; attempt < 200; attempt++) {
      frame = document.querySelector('[data-testid="trigger-editor"]');
      if (frame || document.querySelector('[role="alert"]')) break;
      await new Promise(resolve => setTimeout(resolve, 25));
    }
    require(window.__DANDRUM__ === undefined && window.__JUCE__ === undefined, 'no native transport');
    require(!document.querySelector('[role="alert"]'), 'no error shell');
    require(frame && document.querySelector('.preview') && !document.querySelector('.preview--native'), 'mounted silent preview');
    const backend = document.querySelector('[data-testid="backend-status"]');
    require(backend?.dataset.backend === 'mock' && backend.textContent === 'Silent', 'explicit Silent mock branch');
    require(!document.body.textContent.includes('Opening silent sampler preview…') && !document.body.textContent.includes('Connecting to native sampler…'), 'startup placeholder replaced');
    require(frame.querySelector('header') && frame.querySelector('nav') && frame.querySelector('main'), 'frame structure');
    require([...frame.querySelectorAll('button')].some(button => button.textContent === 'Log'), 'Log control');
    require(frame.querySelector('header').textContent.includes('Untitled') && [...frame.querySelectorAll('button')].some(button => button.textContent === 'Browse samples…') && document.querySelector('input[aria-label="Import sample files"]'), 'sample controls');
    const previewStyle = getComputedStyle(document.querySelector('.preview'));
    const frameStyle = getComputedStyle(frame);
    require(previewStyle.display === 'flex' && previewStyle.paddingTop === '16px', 'production external CSS');
    require(frameStyle.backgroundColor === 'rgb(26, 21, 17)', 'production color token');
    require(frameStyle.width === '1200px' && frameStyle.height === '800px' && frameStyle.display === 'flex', 'React inline frame styles');
    const svg = frame.querySelector('svg');
    require(svg && svg.querySelector('path,rect,line,circle,polyline') && svg.getBoundingClientRect().width > 0 && svg.getBoundingClientRect().height > 0, 'rendered inline SVG');
    report('production-preview|silent|mock|1200px|800px|16px|rgb(26, 21, 17)|inline-svg');
    const fonts = [
      ['Barlow', 500], ['Barlow', 600], ['Barlow', 700],
      ['Barlow Semi Condensed', 500], ['Barlow Semi Condensed', 600], ['Barlow Semi Condensed', 700],
      ['JetBrains Mono', 500], ['JetBrains Mono', 600],
    ];
    for (const [family, weight] of fonts) {
      const faces = await document.fonts.load(`${weight} 16px "${family}"`, 'Dandrum 0123456789');
      require(faces.length > 0 && faces.every(face => face.family.replaceAll('"', '') === family && face.weight === String(weight) && face.status === 'loaded'), `loaded ${family}/${weight}`);
      report(`production-font|${family}|${weight}|loaded`);
    }
    await document.fonts.ready;
    report('production-ready|silent-preview|eight-fonts');
  } catch (error) {
    report(`production-error|${error.message}`);
  }
})();
