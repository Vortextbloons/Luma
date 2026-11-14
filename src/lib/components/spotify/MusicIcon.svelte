<script lang="ts">
  let { name = 'music', size = 24 }: { name?: string; size?: number } = $props();
</script>
<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
{#if name === 'music'}
<path d="M9 18V5l12-2v13" />
  <circle cx="6" cy="18" r="3" />
  <circle cx="18" cy="16" r="3" />
{:else if name === 'play'}
<polygon points="6 3 20 12 6 21 6 3" />
{:else if name === 'pause'}
<rect x="14" y="4" width="4" height="16" rx="1" />
  <rect x="6" y="4" width="4" height="16" rx="1" />
{:else if name === 'skip-back'}
<polygon points="19 20 9 12 19 4 19 20" />
  <line x1="5" x2="5" y1="19" y2="5" />
{:else if name === 'skip-forward'}
<polygon points="5 4 15 12 5 20 5 4" />
  <line x1="19" x2="19" y1="5" y2="19" />
{:else if name === 'search'}
<circle cx="11" cy="11" r="8" />
  <path d="m21 21-4.3-4.3" />
{:else if name === 'x'}
<path d="M18 6 6 18" />
  <path d="m6 6 12 12" />
{:else if name === 'arrow-left'}
<path d="m12 19-7-7 7-7" />
  <path d="M19 12H5" />
{:else if name === 'list-music'}
<path d="M21 15V6" />
  <path d="M18.5 18a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5Z" />
  <path d="M12 12H3" />
  <path d="M16 6H3" />
  <path d="M12 18H3" />
{:else if name === 'library'}
<path d="m16 6 4 14" />
  <path d="M12 6v14" />
  <path d="M8 8v12" />
  <path d="M4 4v16" />
{:else if name === 'speaker'}
<rect width="16" height="20" x="4" y="2" rx="2" />
  <path d="M12 6h.01" />
  <circle cx="12" cy="14" r="4" />
  <path d="M12 14h.01" />
{:else if name === 'shuffle'}
<path d="m18 14 4 4-4 4" />
  <path d="m18 2 4 4-4 4" />
  <path d="M2 18h1.973a4 4 0 0 0 3.3-1.7l5.454-8.6a4 4 0 0 1 3.3-1.7H22" />
  <path d="M2 6h1.972a4 4 0 0 1 3.6 2.2" />
  <path d="M22 18h-6.041a4 4 0 0 1-3.3-1.8l-.359-.45" />
{:else if name === 'repeat'}
<path d="m17 2 4 4-4 4" />
  <path d="M3 11v-1a4 4 0 0 1 4-4h14" />
  <path d="m7 22-4-4 4-4" />
  <path d="M21 13v1a4 4 0 0 1-4 4H3" />
{:else if name === 'chevron-right'}
<path d="m9 18 6-6-6-6" />
{:else if name === 'disc-3'}
<circle cx="12" cy="12" r="10" />
  <path d="M6 12c0-1.7.7-3.2 1.8-4.2" />
  <circle cx="12" cy="12" r="2" />
  <path d="M18 12c0 1.7-.7 3.2-1.8 4.2" />
{:else if name === 'heart'}
<path d="M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.3 1.5 4.05 3 5.5l7 7Z" />
{/if}
</svg>
