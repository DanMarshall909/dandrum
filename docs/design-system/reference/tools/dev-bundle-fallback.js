// Dev fallback: if _ds_bundle.js hasn't been compiled yet, transpile component sources in-browser.
// No-op when the compiled bundle is present.
(function () {
  var NS = 'DandrumDesignSystem_3c2eab';
  if (window[NS] || !window.Babel) return;
  var base = (document.currentScript.src || '').replace(/tools\/dev-bundle-fallback\.js.*$/, '');
  var files = ['components/icons/Icon.jsx', 'components/display/ModIndicator.jsx', 'components/controls/Knob.jsx', 'components/controls/Slider.jsx',
    'components/controls/Button.jsx', 'components/controls/Toggle.jsx', 'components/controls/NumericField.jsx', 'components/navigation/Tabs.jsx',
    'components/navigation/ContextMenu.jsx', 'components/display/PadCell.jsx', 'components/display/WaveformPanel.jsx', 'components/display/KeyMap.jsx', 'components/display/LayerStack.jsx', 'components/display/OutputBusses.jsx', 'components/display/Meter.jsx',
    'components/display/ListRow.jsx', 'components/feedback/StatusMessage.jsx', 'components/layout/Panel.jsx'];
  var out = {};
  files.forEach(function (f) {
    var x = new XMLHttpRequest(); x.open('GET', base + f, false); x.send();
    var src = x.responseText.replace(/^import .*$/mg, '');
    var names = []; src = src.replace(/export (function|const) (\w+)/g, function (_, k, n) { names.push(n); return k + ' ' + n; });
    var code = Babel.transform(src, { presets: ['react'] }).code;
    var fn = new Function('React', 'NS', 'with (NS) {' + code + '\n;return {' + names.join(',') + '};}');
    Object.assign(out, fn(window.React, out));
  });
  window[NS] = out;
})();
