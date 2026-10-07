export function observePanelHeight(panel, frame, Observer = globalThis.ResizeObserver) {
  let closed = false;
  const update = () => {
    if (closed) return;
    const height = `${panel.getBoundingClientRect().height}px`;
    if (frame.style.height !== height) frame.style.height = height;
  };
  const observer = new Observer(update);
  observer.observe(panel);
  update();
  return () => { closed = true; observer.disconnect(); };
}
