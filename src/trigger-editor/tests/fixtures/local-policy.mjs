const report = value => window.ipc.postMessage(value);
document.addEventListener('securitypolicyviolation', event => {
  report([
    'policy', event.effectiveDirective, event.disposition,
    new URL(event.blockedURI).origin, event.documentURI,
  ].join('|'));
});

const response = await fetch('/absent');
const control = getComputedStyle(document.getElementById('control'));
report(['local', response.status, control.height, control.width].join('|'));

try {
  // Opaque no-cors deliberately removes CORS as a reason for rejection. The
  // independent owned endpoint records whether an actual HTTP request arrives.
  await fetch('__LOCAL_POLICY_ENDPOINT__/external-attempt', { mode: 'no-cors' });
  report('external|resolved');
} catch {
  report('external|rejected');
}
// Violation events are queued browser tasks. Report completion on the next task
// after fetch settles so the native observer sees the associated event too.
setTimeout(() => report('done'), 0);
