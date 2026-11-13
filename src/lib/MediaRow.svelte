<script lang="ts">
  let { onselect }: { onselect: (title: string) => void } = $props();
  const artwork = Object.values(import.meta.glob('../assets/*.webp', { eager: true, query: '?url', import: 'default' })) as string[];
  const titles = ['Midnight Horizon', 'The Silent Tide', 'Ember Road', 'Northstar', 'The Citadel'];
</script>
<section class="px-[4.4vw] py-8"><h2 class="text-2xl mb-5">Featured for you <span class="text-sm text-white/50">Demo collection</span></h2><div class="flex gap-5 overflow-x-auto py-4">{#each titles as title, index (title)}<button class="min-w-52 flex-1 text-left" onclick={() => onselect(title)}><img class="aspect-video w-full rounded-xl object-cover mb-3" src={artwork[(index+1) % artwork.length]} alt="" loading="lazy"/>{title}</button>{/each}</div></section>
