<script lang="ts">
  // Smart playlist rules sheet: dims the window and attaches to the top, like a macOS sheet.
  import Icon from './Icon.svelte'
  import { FIELDS, OPS, evaluate, type Field, type LimitBy, type Rule, type Rules } from '../lib/smart'
  import { lib } from '../lib/library.svelte'
  import { plural } from '../lib/format'
  import { untrack } from 'svelte'

  let { name: initialName = '', rules: initial, onsave, oncancel }: {
    name?: string
    rules?: Rules
    onsave: (name: string, rules: Rules) => void
    oncancel: () => void
  } = $props()

  // the sheet edits a copy of the starting values; the props are read once on open
  const start = untrack(() => ({ name: initialName, rules: $state.snapshot(initial) as Rules | undefined }))
  let name = $state(start.name || 'New Smart Playlist')
  let r = $state<Rules>(structuredClone(start.rules) ?? { match: 'all', rules: [{ field: 'genre', op: 'contains', value: '' }] })
  let limited = $state(!!start.rules?.limit)
  let limit = $state(start.rules?.limit ?? { count: 25, by: 'random' as LimitBy })

  const fields = Object.entries(FIELDS) as [Field, (typeof FIELDS)[Field]][]
  const typeOf = (f: Field) => FIELDS[f].type
  const preview = $derived(evaluate(lib.tracks, { ...$state.snapshot(r), limit: limited ? limit : undefined }).length)

  function setField(rule: Rule, f: Field) {
    const changed = typeOf(f) !== typeOf(rule.field)
    rule.field = f
    if (changed) {
      rule.op = OPS[typeOf(f)][0][0]
      rule.value = typeOf(f) === 'date' ? 30 : typeOf(f) === 'num' ? 0 : ''
      rule.unit = typeOf(f) === 'date' ? 'days' : undefined
    }
  }
  const add = () => r.rules.push({ field: 'artist', op: 'contains', value: '' })
  function save() {
    const out: Rules = { ...$state.snapshot(r), limit: limited ? { ...limit } : undefined, seed: start.rules?.seed ?? Math.floor(Math.random() * 2 ** 31) }
    onsave(name.trim() || 'Smart Playlist', out)
  }
  let first: HTMLInputElement
  $effect(() => first?.select())
</script>

<div class="scrim" role="presentation" onclick={oncancel}></div>
<div class="sheet" role="dialog" aria-modal="true" aria-label="Smart playlist rules" tabindex="-1" onkeydown={(e) => e.key === 'Escape' && oncancel()}>
  <input bind:this={first} class="name" bind:value={name} aria-label="Playlist name" />
  <div class="match">
    Match
    <select bind:value={r.match}><option value="all">all</option><option value="any">any</option></select>
    of the following rules:
  </div>

  <div class="rules">
    {#each r.rules as rule, i (i)}
      <div class="rule">
        <select value={rule.field} onchange={(e) => setField(rule, e.currentTarget.value as Field)} aria-label="Field">
          {#each fields as [f, meta] (f)}<option value={f}>{meta.label}</option>{/each}
        </select>
        <select bind:value={rule.op} aria-label="Condition">
          {#each OPS[typeOf(rule.field)] as [op, label] (op)}<option value={op}>{label}</option>{/each}
        </select>
        {#if typeOf(rule.field) === 'text'}
          <input bind:value={rule.value} placeholder="value" aria-label="Value" />
        {:else if typeOf(rule.field) === 'num'}
          <input type="number" bind:value={rule.value} aria-label="Value" />
          {#if rule.op === 'between'}<span>to</span><input type="number" bind:value={rule.value2} aria-label="Upper value" />{/if}
        {:else if typeOf(rule.field) === 'date'}
          <input type="number" min="1" bind:value={rule.value} aria-label="Amount" />
          <select bind:value={rule.unit} aria-label="Unit"><option value="days">days</option><option value="weeks">weeks</option><option value="months">months</option></select>
        {/if}
        <span class="sp"></span>
        <button class="icon-btn" aria-label="Remove rule" onclick={() => r.rules.splice(i, 1)} disabled={r.rules.length === 1}><Icon name="Minus" size={14} weight="bold" /></button>
        <button class="icon-btn" aria-label="Add rule" onclick={add}><Icon name="Plus" size={14} weight="bold" /></button>
      </div>
    {/each}
  </div>

  <label class="limit">
    <input type="checkbox" bind:checked={limited} />
    Limit to
    <input type="number" min="1" bind:value={limit.count} disabled={!limited} aria-label="Limit count" />
    songs, selected by
    <select bind:value={limit.by} disabled={!limited} aria-label="Limit order">
      <option value="random">random</option><option value="mostPlayed">most played</option><option value="leastPlayed">least played</option>
      <option value="recentlyAdded">most recently added</option><option value="recentlyPlayed">most recently played</option>
      <option value="highestRated">highest rating</option><option value="title">title</option>
    </select>
  </label>

  <div class="foot">
    <span class="t-sub">{plural(preview, 'song')} match · updates live</span>
    <button class="btn press" onclick={oncancel}>Cancel</button>
    <button class="btn primary press" onclick={save}>Save</button>
  </div>
</div>

<style>
  .scrim { position: fixed; inset: 0; z-index: 80; background: var(--sheet-dim); animation: fade 200ms; }
  .sheet {
    position: fixed; z-index: 81; top: 0; left: 50%; width: min(760px, 92vw); transform: translateX(-50%);
    padding: 20px 22px 18px; border-radius: 0 0 var(--r-xl) var(--r-xl); background: var(--bg-2); border: 1px solid var(--sep); border-top: 0;
    box-shadow: var(--shadow-pop); animation: drop 280ms var(--ease-out);
  }
  @keyframes drop { from { transform: translate(-50%, -100%); } }
  @keyframes fade { from { opacity: 0; } }
  .name { width: 100%; font-size: 20px; font-weight: 650; letter-spacing: -0.015em; border: 0; background: none; outline: none; padding: 2px 0 10px; border-bottom: 1px solid var(--sep); }
  .match, .limit { display: flex; align-items: center; gap: 8px; margin: 14px 0 10px; }
  .rules { display: grid; gap: 6px; max-height: 46vh; overflow-y: auto; }
  .rule { display: flex; align-items: center; gap: 8px; padding: 6px 8px; border-radius: var(--r); background: var(--fill); }
  .sp { flex: 1; }
  select, input:not([type='checkbox']):not(.name) {
    height: 28px; padding: 0 8px; border-radius: var(--r-sm); border: 1px solid var(--sep); background: var(--bg); color: var(--text);
  }
  input[type='number'] { width: 80px; }
  .rule input:not([type='number']) { flex: 1; min-width: 120px; }
  .foot { display: flex; align-items: center; gap: 10px; margin-top: 18px; }
  .foot .t-sub { flex: 1; }
  @media (prefers-reduced-motion: reduce) { .sheet { animation: fade 150ms; } }
</style>
