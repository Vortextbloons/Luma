export function smoothHeight(element: HTMLElement) {
  const content = element.firstElementChild;
  if (!(content instanceof HTMLElement)) return;
  const preference = window.matchMedia('(prefers-reduced-motion: reduce)');
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
    if (preference.matches) return;
    element.style.overflow = 'clip';
    animation = element.animate([{ height: `${previous}px` }, { height: `${next}px` }], {
      duration: 180,
      easing: 'cubic-bezier(.22,.8,.24,1)',
    });
    animation.onfinish = cancel;
  });
  observer.observe(content);
  preference.addEventListener('change', cancel);
  return () => {
    observer.disconnect();
    preference.removeEventListener('change', cancel);
    cancel();
  };
}
