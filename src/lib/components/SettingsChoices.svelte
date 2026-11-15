<script lang="ts">
  import { applySettingValue, settingChoices, settingValue, type SettingChoice } from '../services/settings';
  import type { Settings } from '../types';

  let { settings, section, disabledChoice }: {
    settings: Settings;
    section: string;
    disabledChoice?: (choice: SettingChoice) => boolean;
  } = $props();

  const choices = $derived(settingChoices.filter(choice => choice.section === section));
  const selects = $derived(choices.filter(choice => choice.kind === 'choice'));
  const toggles = $derived(choices.filter(choice => choice.kind === 'toggle'));
</script>

{#if selects.length}
  <div class="settings-form-grid">
    {#each selects as choice (choice.key)}
      <label class="field">{choice.label}
        <select
          value={settingValue(settings, choice.key)}
          disabled={disabledChoice?.(choice) ?? false}
          onchange={(event) => applySettingValue(settings, choice.key, event.currentTarget.value)}
        >
          {#each choice.options as option (option.value)}<option value={option.value}>{option.label}</option>{/each}
        </select>
        <span class="field-hint">{choice.description}</span>
      </label>
    {/each}
  </div>
{/if}

{#if toggles.length}
  <div class="settings-toggle-list">
    {#each toggles as choice (choice.key)}
      <label class="setting-toggle">
        <span class="setting-toggle-copy"><span>{choice.label}</span><small>{choice.description}</small></span>
        <input
          type="checkbox"
          checked={settingValue(settings, choice.key) === 'true'}
          disabled={disabledChoice?.(choice) ?? false}
          onchange={(event) => applySettingValue(settings, choice.key, String(event.currentTarget.checked))}
        />
      </label>
    {/each}
  </div>
{/if}
