<script lang="ts">
  import brand from "../assets/brand.png?inline";

  let { className = "size-6", wordmark = true, tag = true }: { className?: string; wordmark?: boolean; tag?: boolean } = $props();
</script>

<span class="flex min-w-0 items-center gap-2" aria-label="Luma">
  <img src={brand} alt="" class={`${className} flex-none rounded-[7px] object-contain shadow-[0_3px_12px_rgba(0,0,0,.45)]`} />
  {#if wordmark}
    <span class="flex items-end gap-1.5 leading-none">
      <span class="text-[17px] font-semibold tracking-[-0.035em] text-ink">luma</span>
      {#if tag}<span class="pb-px text-[9px] font-bold tracking-[0.16em] text-faint">TV</span>{/if}
    </span>
  {/if}
</span>
