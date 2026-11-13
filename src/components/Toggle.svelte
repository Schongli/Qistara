<script lang="ts">
  let { checked = $bindable(false), label, hint = '', disabled = false }: { checked?: boolean; label: string; hint?: string; disabled?: boolean } = $props()
</script>

<label class="row" class:disabled>
  <span class="txt">
    <span>{label}</span>
    {#if hint}<span class="hint">{hint}</span>{/if}
  </span>
  <input type="checkbox" role="switch" bind:checked {disabled} />
</label>

<style>
  .row { display: flex; align-items: center; justify-content: space-between; gap: 16px; min-height: 46px; padding: 8px 14px; border-bottom: 1px solid var(--sep); }
  .row:last-child { border-bottom: 0; }
  .row.disabled { opacity: 0.5; }
  .txt { display: grid; gap: 2px; }
  .hint { font-size: 12px; color: var(--text-2); line-height: 1.4; }
  input {
    appearance: none; flex: none; width: 38px; height: 22px; margin: 0; border-radius: 11px; position: relative;
    background: var(--fill-2); transition: background-color 180ms var(--ease-out);
  }
  input::after {
    content: ''; position: absolute; top: 2px; left: 2px; width: 18px; height: 18px; border-radius: 50%; background: #fff;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3); transition: transform 220ms cubic-bezier(0.3, 1.4, 0.5, 1);
  }
  input:checked { background: var(--accent); }
  input:checked::after { transform: translateX(16px); }
  input:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
</style>
