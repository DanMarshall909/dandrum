const report = value => window.ipc.postMessage(value);
const response = await fetch('/absent');
const control = getComputedStyle(document.getElementById('control'));
report(['local', response.status, control.height, control.width].join('|'));

if (location.href === 'dandrum://localhost/index.html') {
  const target = '__NAVIGATION_TARGET__';
  report(`navigation-attempt|${target}`);
  location.assign(target);
  // A continued local task/fetch shows the original document remains operational
  // after the ordinary page's navigation request has been handled.
  setTimeout(async () => {
    const response = await fetch('/absent');
    report(`navigation-survived|${response.status}|${location.href}`);
  }, 100);
} else {
  report(`alternate-document|${location.href}`);
}
