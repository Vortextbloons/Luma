<script lang="ts">
  import type { WidgetProps } from '../../services/widgetModel';
  let { config, context, compact = false }: WidgetProps = $props();
  let now = $state(new Date());
  $effect(() => {
    now = new Date();
    const timer = setInterval(() => now = new Date(), 1000);
    return () => clearInterval(timer);
  });
  const hour12 = $derived(config.settings.hour12 === true);
  const showDate = $derived(config.settings.showDate !== false && !compact);
  const time = $derived(now.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit', hour12 }));
  const date = $derived(now.toLocaleDateString([], { weekday: 'short', month: 'short', day: 'numeric' }));
</script>

<aside
  class={['clock-widget', `is-${config.size}`, { 'is-compact': compact, 'is-idle': context === 'idle' }]}
  data-variant={config.variant}
  aria-label="Clock"
>
  <span class="clock-time">{time}</span>
  {#if showDate}<span class="clock-date">{date}</span>{/if}
</aside>

<style>
  .clock-widget { display: grid; gap: 2px; min-width: 0; color: var(--text); }
  .clock-time { font-size: clamp(26px, 2.6vw, 42px); font-weight: 650; line-height: 1; letter-spacing: -.04em; font-variant-numeric: tabular-nums; }
  .clock-date { color: var(--text-dim); font-size: clamp(11px, .82vw, 14px); font-weight: 550; letter-spacing: .01em; }
  .clock-widget[data-variant="glass"] { padding: 14px 18px; border: 1px solid var(--line-strong); border-radius: 14px; background: color-mix(in srgb, var(--surface-2) 92%, transparent); box-shadow: 0 14px 38px #0006; backdrop-filter: blur(16px); }
  .clock-widget[data-variant="cinematic"] { padding: 16px 20px; border-radius: 16px; background: linear-gradient(135deg, color-mix(in srgb, var(--surface-2) 82%, var(--accent) 18%), var(--surface-2)); border: 1px solid color-mix(in srgb, var(--accent) 34%, var(--line)); box-shadow: 0 18px 50px #0007; }
  .clock-widget[data-variant="cinematic"] .clock-time { color: var(--accent); }
  .clock-widget[data-variant="ticker"] { display: flex; align-items: baseline; gap: 10px; padding: 8px 14px; border-radius: 99px; background: color-mix(in srgb, var(--surface-2) 92%, transparent); border: 1px solid var(--line); }
  .clock-widget[data-variant="ticker"] .clock-time { font-size: clamp(18px, 1.5vw, 24px); }
  .clock-widget[data-variant="minimal"] { padding: 4px 2px; }
  .clock-widget.is-small .clock-time { font-size: clamp(22px, 2vw, 32px); }
  .clock-widget.is-large .clock-time { font-size: clamp(34px, 3.6vw, 60px); }
  .clock-widget.is-compact { display: flex; align-items: baseline; gap: 10px; padding: 6px 12px; border-radius: 99px; background: #ffffff0d; border: 1px solid #ffffff1f; }
  .clock-widget.is-compact .clock-time { font-size: 15px; }
</style>
