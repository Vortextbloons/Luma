export function slideIndicator(element: HTMLElement, options: { selector?: string; key?: unknown } = {}) {
  const selector = options.selector ?? '.is-active';
  let frame = 0;
  function measure() {
    frame = 0;
    const target = element.querySelector<HTMLElement>(selector);
    if (!target || !target.getClientRects().length) {
      delete element.dataset.indicator;
      return;
    }
    const box = element.getBoundingClientRect();
    const rect = target.getBoundingClientRect();
    element.style.setProperty('--indicator-x', `${rect.left - box.left - element.clientLeft}px`);
    element.style.setProperty('--indicator-y', `${rect.top - box.top - element.clientTop}px`);
    element.style.setProperty('--indicator-w', `${rect.width}px`);
    element.style.setProperty('--indicator-h', `${rect.height}px`);
    element.dataset.indicator = 'ready';
  }
  function update() {
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(measure);
  }
  const resize = new ResizeObserver(update);
  resize.observe(element);
  const mutation = new MutationObserver(update);
  mutation.observe(element, { childList: true, subtree: true, attributes: true, attributeFilter: ['class'] });
  window.addEventListener('resize', update);
  if (document.fonts) void document.fonts.ready.then(update);
  update();
  return {
    update,
    destroy() {
      cancelAnimationFrame(frame);
      resize.disconnect();
      mutation.disconnect();
      window.removeEventListener('resize', update);
    },
  };
}

export function smoothHeight(element: HTMLElement) {
  const content = element.firstElementChild;
  if (!(content instanceof HTMLElement)) return;
  const preference = window.matchMedia('(prefers-reduced-motion: reduce)');
  const reduced = () => preference.matches || (typeof document !== 'undefined' && document.documentElement.dataset.reducedMotion === 'true');
  const overflow = element.style.overflow;
  let height = content.getBoundingClientRect().height;
  let animation: Animation | undefined;
  function cancel() {
    animation?.cancel();
    animation = undefined;
    element.style.overflow = overflow;
  }
  const observer = new ResizeObserver(() => {
    const next = content.getBoundingClientRect().height;
    if (Math.abs(next - height) < 1) return;
    const previous = animation ? element.getBoundingClientRect().height : height;
    height = next;
    cancel();
    if (reduced() || Math.abs(next - previous) > 320) return;
    element.style.overflow = 'clip';
    animation = element.animate([{ height: `${previous}px` }, { height: `${next}px` }], {
      duration: 180,
      easing: 'cubic-bezier(.22,.8,.24,1)',
    });
    animation.onfinish = cancel;
  });
  const settings = typeof MutationObserver !== 'undefined' && typeof document !== 'undefined'
    ? new MutationObserver(() => { if (reduced()) cancel(); })
    : undefined;
  settings?.observe(document.documentElement, { attributes: true, attributeFilter: ['data-reduced-motion'] });
  observer.observe(content);
  preference.addEventListener('change', cancel);
  return () => {
    observer.disconnect();
    settings?.disconnect();
    preference.removeEventListener('change', cancel);
    cancel();
  };
}
