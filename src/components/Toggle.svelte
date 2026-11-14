<script lang="ts">
  import Switch from './Switch.svelte'

  let { checked = $bindable(false), label, hint = '', disabled = false }: { checked?: boolean; label: string; hint?: string; disabled?: boolean } = $props()
</script>

<label class="row" class:disabled>
  <span class="txt">
    <span>{label}</span>
    {#if hint}<span class="hint">{hint}</span>{/if}
  </span>
  <Switch bind:checked {disabled} />
</label>

<style>
  .row { display: flex; align-items: center; justify-content: space-between; gap: 16px; min-height: 46px; padding: 8px 14px; border-bottom: 1px solid var(--sep); }
  .row:last-child { border-bottom: 0; }
  .row.disabled { opacity: 0.5; }
  .txt { display: grid; gap: 2px; }
  .hint { font-size: 12px; color: var(--text-2); line-height: 1.4; }
</style>
