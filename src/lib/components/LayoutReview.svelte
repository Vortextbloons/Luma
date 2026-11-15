<script lang="ts">
  let { busy, editing, keep, cancel, error = '' }: { busy: boolean; editing: boolean; keep: () => void; cancel: () => void; error?: string } = $props();
  function mount(element: HTMLElement) { element.querySelector<HTMLButtonElement>('button')?.focus({ preventScroll: true }); }
</script>

<section class="layout-review" aria-label="Layout preview" {@attach mount}>
  <div><strong>Previewing your layout</strong><span>Your saved layout is unchanged until you keep this preview.</span>{#if error}<p role="alert">{error}</p>{/if}</div>
  <button class="panel-secondary is-primary" disabled={busy} onclick={keep}>{busy ? 'Please wait…' : 'Keep layout'}</button>
  <button class="panel-secondary" disabled={busy} onclick={cancel}>{editing ? 'Back to editor' : 'Cancel preview'}</button>
</section>

<style>
  .layout-review { position: fixed; z-index: 120; inset: auto var(--side) 24px; display: flex; align-items: center; gap: 18px; padding: 22px 26px; border: 1px solid var(--line-strong); border-radius: 16px; background: var(--surface); box-shadow: 0 14px 40px #0008; }
  .layout-review div { flex: 1; display: grid; gap: 6px; }
  .layout-review strong { color: var(--text); font-size: clamp(18px, 1.3vw, 26px); }
  .layout-review span { color: var(--text-dim); font-size: clamp(14px, 1vw, 20px); }
  .layout-review p { margin: 4px 0 0; color: var(--warn); font-size: clamp(14px, 1vw, 20px); }
  .layout-review button { min-height: 52px; font-size: clamp(15px, 1vw, 20px); padding: 12px 22px; }
  @media (max-width: 800px) { .layout-review { flex-wrap: wrap; } .layout-review div { flex-basis: 100%; } }
</style>
