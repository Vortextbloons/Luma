<script lang="ts">
  import type { MediaItem } from '../types';
  let { item, close, launch, message }: { message: string; item: MediaItem; close: () => void; launch: () => void } = $props();
  function mountDialog(dialog: HTMLDialogElement) { dialog.showModal(); }
</script>

<dialog {@attach mountDialog} class="details-dialog" aria-labelledby="details-title" oncancel={(event) => { event.preventDefault(); close(); }}>
  <button class="close-btn float-right mb-4" onclick={close} aria-label="Close details">×</button>
  <img src={item.artwork} alt="" />
  <h2 id="details-title">{item.title}</h2>
  <p>{item.subtitle}</p>
  {#if item.description}<p>{item.description}</p>{/if}
  <p>Luma does not stream this title. Open Stremio on your PC to watch it.</p>
  {#if message}<p role="status">{message}</p>{/if}
  <button class="btn btn-primary" onclick={launch}>Open Stremio</button>
</dialog>
