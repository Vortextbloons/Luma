<script lang="ts">
  import { tick } from "svelte";
  import Icon from "./Icon.svelte";
  import Modal from "./Modal.svelte";
  import { apps, loadApps } from "../lib/apps.svelte";
  import { api } from "../lib/api";
  import { applyState, showPairing } from "../lib/session.svelte";
  import { say } from "../lib/toast.svelte";

  let { open = false, onclose }: { open?: boolean; onclose: () => void } = $props();

  let order = $state<string[]>([]);
  let note = $state("");
  let saving = $state(false);
  let queued = false;
  let timer: ReturnType<typeof setTimeout> | null = null;
  let wasOpen = false;

  $effect(() => {
    if (open && !wasOpen) {
      wasOpen = true;
      void start();
    } else if (!open) {
      wasOpen = false;
      if (timer) clearTimeout(timer);
      timer = null;
    }
  });

  async function start(): Promise<void> {
    if (!apps.list.length) await loadApps();
    if (!apps.list.length) {
      say("The app list is not available yet.", true);
      onclose();
      return;
    }
    order = apps.list.map(app => app.name).filter(name => name !== "Custom");
    order.push("Custom");
    note = "";
    await tick();
  }

  function iconFor(name: string): string | undefined {
    return apps.list.find(app => app.name === name)?.icon;
  }

  function move(index: number, direction: number): void {
    const target = index + direction;
    if (target < 0 || target >= order.length - 1) return;
    const next = [...order];
    [next[index], next[target]] = [next[target], next[index]];
    order = next;
    note = "Saving…";
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => void save(), 250);
  }

  async function save(): Promise<void> {
    if (saving) {
      queued = true;
      return;
    }
    saving = true;
    try {
      const { response, data } = await api.layout(order);
      if (response.status === 401) {
        onclose();
        showPairing("This remote is no longer paired. Choose Connect in Luma for a new code.");
        return;
      }
      if (!response.ok || data?.error) {
        throw new Error(data?.error || "Could not save the app order.");
      }
      if (data?.state) applyState(data.state);
      note = "Saved to the TV and Display Box.";
      void loadApps(true);
    } catch (error) {
      note = error instanceof Error ? error.message : "Could not save the app order.";
    } finally {
      saving = false;
      if (queued) {
        queued = false;
        if (timer) clearTimeout(timer);
        timer = setTimeout(() => void save(), 100);
      }
    }
  }
</script>

<Modal
  id="layout-dialog"
  titleId="layout-title"
  {open}
  className="layout-dialog m-auto max-h-[min(86dvh,720px)] w-[min(420px,calc(100%-32px))] overflow-hidden rounded-[24px] border border-line-strong bg-surface-2 shadow-[0_30px_80px_rgba(0,0,0,.55)]"
  onrequestclose={onclose}
>
  <header class="flex items-center justify-between gap-3 border-b border-line px-5 pb-3 pt-5">
    <div>
      <h2 id="layout-title" class="text-[20px] font-semibold tracking-[-0.03em] text-ink">App order</h2>
      <p class="mt-1 text-[12px] text-faint">This order is shared with the TV and Display Box.</p>
    </div>
    <button
      type="button"
      class="grid size-10 flex-none place-items-center rounded-full border border-line bg-white/[0.045] text-muted transition active:scale-95"
      aria-label="Close app order"
      onclick={onclose}
    >
      <Icon name="close" className="size-[15px]" />
    </button>
  </header>

  <div class="min-h-0 flex-1 overflow-y-auto overscroll-contain px-4 py-3">
    <div class="grid gap-1.5">
      {#each order as name, index (name)}
        <div class="flex items-center gap-3 rounded-xl border border-line bg-white/[0.025] px-2.5 py-2">
          <img src={iconFor(name)} alt="" width="26" height="26" class="size-[26px] flex-none object-contain" />
          <span class="min-w-0 flex-1">
            <b class="block truncate text-[13px] font-semibold text-ink-dim">{name}</b>
            <small class="mt-0.5 block text-[10.5px] text-faint">
              {name === "Custom" ? "Fixed last" : `Position ${index + 1}`}
            </small>
          </span>

          {#if name === "Custom"}
            <span class="rounded-full border border-line px-2 py-1 text-[9.5px] font-bold text-faint">Fixed</span>
          {:else}
            <span class="flex flex-none gap-1">
              <button
                type="button"
                class="grid size-11 place-items-center rounded-lg border border-line-strong bg-white/[0.05] text-ink-dim transition active:bg-white/[0.12] disabled:opacity-30"
                aria-label={`Move ${name} up`}
                disabled={index === 0}
                onclick={() => move(index, -1)}
              >
                <Icon name="up" className="size-[15px]" />
              </button>
              <button
                type="button"
                class="grid size-11 place-items-center rounded-lg border border-line-strong bg-white/[0.05] text-ink-dim transition active:bg-white/[0.12] disabled:opacity-30"
                aria-label={`Move ${name} down`}
                disabled={index >= order.length - 2}
                onclick={() => move(index, 1)}
              >
                <Icon name="down" className="size-[15px]" />
              </button>
            </span>
          {/if}
        </div>
      {/each}
    </div>
    <p class="mt-3 min-h-5 px-1 text-[11.5px] leading-relaxed text-muted" role="status" aria-live="polite">{note}</p>
  </div>

  <footer class="flex justify-end border-t border-line px-5 py-3">
    <button
      type="button"
      class="min-h-10 rounded-full border border-line-strong bg-white/[0.07] px-4 text-[12.5px] font-semibold text-ink transition active:scale-[.97]"
      onclick={onclose}
    >
      Done
    </button>
  </footer>
</Modal>
