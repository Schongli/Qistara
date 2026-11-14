<script lang="ts">
  // The on/off switch on its own; Toggle wraps it in a settings row with a label.
  let { checked = $bindable(false), disabled = false, label }: { checked?: boolean; disabled?: boolean; label?: string } = $props()
</script>

<input type="checkbox" role="switch" bind:checked {disabled} aria-label={label} />

<style>
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
  input:disabled { opacity: 0.5; }
</style>
