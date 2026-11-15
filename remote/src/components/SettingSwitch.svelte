<script lang="ts">
  let {
    checked = false,
    label,
    disabled = false,
    onchange,
  }: {
    checked?: boolean;
    label: string;
    disabled?: boolean;
    onchange: (checked: boolean) => void;
  } = $props();

  let track = $derived(
    `relative h-7 w-12 flex-none rounded-full border transition ${
      checked ? "border-transparent bg-accent" : "border-line-strong bg-white/[0.12]"
    } ${disabled ? "opacity-50" : ""}`,
  );
  let knob = $derived(
    `absolute left-0.5 top-0.5 size-6 rounded-full shadow-[0_1px_4px_rgba(0,0,0,.45)] transition-transform ${
      checked ? "translate-x-5 bg-accent-ink" : "bg-ink"
    }`,
  );
</script>

<button
  type="button"
  role="switch"
  aria-checked={checked}
  aria-label={label}
  {disabled}
  class={track}
  onclick={() => onchange(!checked)}
>
  <span class={knob}></span>
</button>
