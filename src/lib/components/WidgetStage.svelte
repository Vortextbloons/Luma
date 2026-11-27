<script lang="ts">
  import WidgetHost from './WidgetHost.svelte';
  import WidgetHud from './WidgetHud.svelte';
  import type { Settings, WidgetContext } from '../types';
  let { widgets, context, selected, onselect }: {
    widgets: Settings['widgets'];
    context: WidgetContext;
    selected: string | null;
    onselect: (id: string) => void;
  } = $props();
  let width = $state(0);
  const scale = $derived(width > 0 ? width / 1280 : 0.5);
</script>

<div class="widget-stage" bind:clientWidth={width} class:is-music={context === 'music'} class:is-idle={context === 'idle'}>
  <div class="stage-screen" style:transform={`scale(${scale})`}>
    <div class="stage-ambient" aria-hidden="true"></div>
    {#if context === 'music'}
      <div class="stage-music" aria-hidden="true">
        <span class="stage-art"></span>
        <span class="stage-copy"><i></i><i class="short"></i><i class="shorter"></i></span>
      </div>
      <div class="stage-hud"><WidgetHud {widgets} {selected} onselect={onselect} preview onopen={() => {}} /></div>
    {:else}
      <div class="stage-top" aria-hidden="true"><span class="stage-logo"></span><span class="stage-pills"><i class="active"></i><i></i><i></i><i></i></span><span class="stage-clock"></span></div>
      <div class="stage-hero" aria-hidden="true">
        <span class="stage-hero-copy"><i class="eyebrow"></i><i class="title"></i><i class="meta"></i><i class="actions"></i></span>
      </div>
      <div class="stage-rows" aria-hidden="true">
        {#each [0, 1, 2] as row}
          <div class="stage-row"><i></i><span>{#each [0, 1, 2, 3] as tile}<b></b>{/each}</span></div>
        {/each}
      </div>
      <WidgetHost contained={true} {context} {widgets} preview={true} interactive={false} {selected} {onselect} onopen={() => {}} />
    {/if}
  </div>
</div>

<style>
  .widget-stage { position: relative; width: 100%; aspect-ratio: 16 / 9; overflow: hidden; border: 1px solid var(--line-strong); border-radius: 18px; background: #0b0d13; box-shadow: 0 20px 60px #0008; container-type: inline-size; }
  .stage-screen { position: relative; width: 1280px; height: 720px; transform-origin: top left; overflow: hidden; background: radial-gradient(120% 90% at 50% 0%, color-mix(in srgb, var(--surface-3) 55%, var(--bg)), var(--bg) 62%); }
  .stage-ambient { position: absolute; inset: 0; background: radial-gradient(70% 60% at 22% 12%, color-mix(in srgb, var(--accent) 16%, transparent), transparent 70%), radial-gradient(60% 55% at 82% 18%, color-mix(in srgb, var(--surface-3) 40%, transparent), transparent 72%); }
  .is-music .stage-ambient { background: radial-gradient(60% 70% at 18% 18%, color-mix(in srgb, var(--accent) 30%, transparent), transparent 70%), linear-gradient(180deg, #0b0d13, #07080c); }
  .is-idle .stage-ambient { background: radial-gradient(70% 70% at 50% 10%, color-mix(in srgb, var(--accent) 12%, transparent), transparent 72%), linear-gradient(180deg, #0a0b10, #06070a); }
  .stage-top { position: absolute; inset: 26px 44px auto; display: flex; align-items: center; gap: 30px; color: var(--text-dim); }
  .stage-logo { width: 32px; height: 32px; border-radius: 9px; background: var(--accent); opacity: .9; }
  .stage-pills { display: flex; gap: 22px; }
  .stage-pills i { width: 68px; height: 9px; border-radius: 99px; background: #ffffff1c; }
  .stage-pills i.active { background: #ffffff45; }
  .stage-clock { margin-left: auto; width: 58px; height: 10px; border-radius: 99px; background: #ffffff24; }
  .stage-hero { position: absolute; left: 44px; right: 44px; top: 100px; height: 250px; border-radius: 18px; background: linear-gradient(120deg, #ffffff12, #ffffff05 55%, transparent); border: 1px solid #ffffff10; overflow: hidden; }
  .stage-hero:after { content: ''; position: absolute; inset: 0; background: linear-gradient(90deg, #00000055, transparent 60%); }
  .stage-hero-copy { position: absolute; left: 46px; bottom: 34px; display: grid; gap: 13px; width: 34%; z-index: 1; }
  .stage-hero-copy i { display: block; border-radius: 99px; background: #ffffff32; height: 10px; }
  .stage-hero-copy .eyebrow { width: 30%; height: 7px; opacity: .7; }
  .stage-hero-copy .title { width: 92%; height: 26px; background: #ffffff55; }
  .stage-hero-copy .meta { width: 58%; height: 9px; opacity: .55; }
  .stage-hero-copy .actions { width: 46%; height: 26px; border-radius: 10px; background: #ffffff30; }
  .stage-rows { position: absolute; left: 44px; right: 44px; top: 384px; display: grid; gap: 22px; }
  .stage-row > i { display: block; width: 132px; height: 9px; border-radius: 99px; background: #ffffff2b; margin-bottom: 10px; }
  .stage-row > span { display: flex; gap: 12px; }
  .stage-row b { display: block; width: 128px; aspect-ratio: 16 / 9; border-radius: 9px; background: linear-gradient(135deg, #ffffff12, #ffffff06); border: 1px solid #ffffff0d; }
  .is-music .stage-music { position: absolute; inset: 96px 120px 150px; display: flex; align-items: center; gap: 64px; }
  .stage-art { width: 330px; height: 330px; border-radius: 16px; background: linear-gradient(140deg, color-mix(in srgb, var(--surface-3) 80%, var(--accent) 20%), var(--surface-2)); box-shadow: 0 30px 80px #000a; }
  .stage-copy { display: grid; gap: 18px; width: 52%; }
  .stage-copy i { display: block; height: 22px; border-radius: 99px; background: #ffffff2e; }
  .stage-copy i.short { width: 62%; height: 14px; opacity: .6; }
  .stage-copy i.shorter { width: 40%; height: 11px; opacity: .4; }
  .stage-hud { position: absolute; left: 0; right: 0; bottom: 38px; }
  .stage-hud :global(.widget-hud) { margin-top: 0; }
  .stage-hud :global(.hud-widget.is-selected) { outline-color: var(--accent); }
  .is-idle .stage-hero, .is-idle .stage-rows { opacity: .24; }
  .is-idle .stage-top { opacity: .35; }
</style>
