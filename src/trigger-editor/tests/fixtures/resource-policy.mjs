const report = value => window.ipc.postMessage(value);
document.addEventListener('securitypolicyviolation', event => {
  report(['resource-policy', event.effectiveDirective, event.violatedDirective,
    event.disposition, event.blockedURI, event.documentURI].join('|'));
});

const local = await fetch('/absent');
const control = getComputedStyle(document.getElementById('control'));
report(['resource-local', local.status, control.height, control.width].join('|'));
const endpoint = '__RESOURCE_POLICY_ENDPOINT__';
const externalControl = document.createElement('div');
externalControl.id = 'external-control';
externalControl.style.height = '7px';
document.body.append(externalControl);

function loadElement(element, parent) {
  return new Promise(resolve => {
    element.onload = () => resolve('loaded');
    element.onerror = () => resolve('rejected');
    parent.append(element);
  });
}
const script = document.createElement('script');
script.src = `${endpoint}/external.js`;
const stylesheet = document.createElement('link');
stylesheet.rel = 'stylesheet';
stylesheet.href = `${endpoint}/external.css`;
const image = new Image();
image.src = `${endpoint}/external.png`;
const font = new FontFace('ResourcePolicyExternal', `url(${endpoint}/external.ttf)`, {weight: '500'});
const attempts = [
  loadElement(script, document.head).then(status =>
    report(['resource-script', status, window.externalResourceMarker ?? 'absent'].join('|'))),
  loadElement(stylesheet, document.head).then(status =>
    report(['resource-style', status, getComputedStyle(externalControl).height].join('|'))),
  font.load().then(face => {
    document.fonts.add(face);
    report(['resource-font', face.status, face.family, face.weight, document.fonts.has(face)].join('|'));
  }, () => report(['resource-font', font.status, font.family, font.weight, document.fonts.has(font)].join('|'))),
  loadElement(image, document.body).then(status =>
    report(['resource-image', status, image.naturalWidth, image.naturalHeight].join('|'))),
];
await Promise.all(attempts);
// Queued CSP events must be delivered before the aggregate receipt is inspected.
setTimeout(() => report('resource-done'), 0);
