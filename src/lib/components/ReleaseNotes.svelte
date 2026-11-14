<script lang="ts">
  type Block = { kind: 'heading' | 'bullet' | 'text'; text: string };
  let { notes }: { notes: string } = $props();
  const blocks = $derived(notes.split(/\r?\n/).flatMap<Block>(line => {
    const text = line.trim();
    if (!text) return [];
    const heading = text.match(/^#{1,6}\s+(.+)$/);
    if (heading) return [{ kind: 'heading', text: heading[1] }];
    const bullet = text.match(/^[-*]\s+(.+)$/);
    return [{ kind: bullet ? 'bullet' : 'text', text: bullet ? bullet[1] : text }];
  }));
</script>

<div class="release-notes">
  {#each blocks as block}
    {#if block.kind === 'heading'}<h4>{block.text}</h4>
    {:else}<p class:release-item={block.kind === 'bullet'}>{block.text}</p>{/if}
  {/each}
</div>

<style>
  .release-notes { display: grid; gap: 5px; font-size: 13px; line-height: 1.5; color: var(--text); overflow-wrap: anywhere; }
  .release-notes h4 { margin: 9px 0 1px; font-size: 11px; font-weight: 700; letter-spacing: .08em; text-transform: uppercase; color: #a9aab5; }
  .release-notes h4:first-child { margin-top: 0; }
  .release-notes p { margin: 0; }
  .release-item { position: relative; padding-left: 17px; }
  .release-item::before { content: '•'; position: absolute; left: 2px; color: var(--accent); }
</style>
