<script lang="ts">
  // "Qur'an: Choose Surah…": says which surah(s) a recording recites when its name doesn't, or that it
  // isn't a recitation at all. Saved per file (quran.svelte.ts overrideRecitation).
  import { SURAHS, latinWords, normalizeArabic, type Recitation, type Riwayah } from '../lib/quran'
  import { overrideRecitation, recitationFor } from '../lib/quran.svelte'
  import type { Track } from '../lib/library.svelte'

  let { track, onclose }: { track: Track; onclose: () => void } = $props()

  const span = (a: number, b: number) => Array.from({ length: b - a + 1 }, (_, i) => a + i)
  const JUZ = [
    { key: 'j30', label: "Juz 'Amma (78–114)", ar: 'جزء عمّ', surahs: span(78, 114) },
    { key: 'j29', label: 'Juz Tabarak (67–77)', ar: 'جزء تبارك', surahs: span(67, 77) },
  ]

  let q = $state('')
  let picked = $state<string>('') // "s<n>" or a JUZ key
  let riwayah = $state<Riwayah>('hafs')
  let partial = $state(false)

  // start from what Qistara thinks now
  $effect(() => {
    recitationFor(track).then((r) => {
      if (!r) return
      riwayah = r.riwayah
      partial = r.partial
      picked = r.surahs.length > 1 ? (JUZ.find((j) => j.surahs[0] === r.surahs[0])?.key ?? '') : `s${r.surahs[0]}`
    })
  })

  const matches = $derived.by(() => {
    const s = q.trim()
    if (!s) return SURAHS
    const lat = latinWords(s).join(' ')
    const ar = normalizeArabic(s)
    return SURAHS.filter((x) => String(x.n) === s || (lat && latinWords(x.en).join(' ').includes(lat)) || (ar && normalizeArabic(x.ar).includes(ar)))
  })

  async function save() {
    const juz = JUZ.find((j) => j.key === picked)
    const surahs = juz ? juz.surahs : picked ? [Number(picked.slice(1))] : []
    if (!surahs.length) return
    const value: Recitation = { surahs, riwayah, partial }
    await overrideRecitation(track, value)
    onclose()
  }
  async function set(value: 'none' | undefined) {
    await overrideRecitation(track, value)
    onclose()
  }
</script>

<div class="scrim" role="presentation" onclick={onclose}></div>
<div class="sheet" role="dialog" aria-modal="true" aria-label="Choose surah" tabindex="-1" onkeydown={(e) => e.key === 'Escape' && onclose()}>
  <h2 class="t-h2">What does this recording recite?</h2>
  <p class="t-sub ellipsis">{track.title}</p>
  <input class="search" bind:value={q} placeholder="Surah name or number" aria-label="Find a surah" spellcheck="false" />
  <div class="list" role="listbox" aria-label="Surahs">
    {#if !q.trim()}
      {#each JUZ as j (j.key)}
        <button class="row press" class:sel={picked === j.key} role="option" aria-selected={picked === j.key} onclick={() => (picked = j.key)}>
          <span class="n">—</span><span class="en">{j.label}</span><span class="ar" lang="ar">{j.ar}</span>
        </button>
      {/each}
    {/if}
    {#each matches as s (s.n)}
      <button class="row press" class:sel={picked === `s${s.n}`} role="option" aria-selected={picked === `s${s.n}`} onclick={() => (picked = `s${s.n}`)}>
        <span class="n">{s.n}</span><span class="en">{s.en}</span><span class="ar" lang="ar">{s.ar}</span>
      </button>
    {/each}
  </div>
  <div class="opts">
    <label>Riwayah
      <select bind:value={riwayah}>
        <option value="hafs">Hafs 'an 'Asim</option>
        <option value="shubah">Shu'bah 'an 'Asim</option>
      </select>
    </label>
    <label class="check"><input type="checkbox" bind:checked={partial} />Only some ayahs (sync by tapping)</label>
  </div>
  <div class="actions">
    <button class="btn press" onclick={() => set(undefined)} title="Let Qistara work it out from the name">Automatic</button>
    <button class="btn press" onclick={() => set('none')}>Not a Recitation</button>
    <span class="sp"></span>
    <button class="btn press" onclick={onclose}>Cancel</button>
    <button class="btn primary press" disabled={!picked} onclick={save}>Save</button>
  </div>
</div>

<style>
  .scrim { position: fixed; inset: 0; z-index: 80; background: var(--sheet-dim); animation: fade 200ms; }
  .sheet {
    position: fixed; z-index: 81; top: 0; left: 50%; width: min(560px, 92vw); transform: translateX(-50%);
    padding: 20px 22px 18px; border-radius: 0 0 var(--r-xl) var(--r-xl); background: var(--bg-2); border: 1px solid var(--sep); border-top: 0;
    box-shadow: var(--shadow-pop); animation: drop 280ms var(--ease-out);
  }
  @keyframes drop { from { transform: translate(-50%, -100%); } }
  @keyframes fade { from { opacity: 0; } }
  h2 { margin: 0; }
  p { margin: 2px 0 12px; }
  .search { width: 100%; height: 30px; padding: 0 10px; border-radius: var(--r); border: 1px solid var(--sep); background: var(--bg); color: var(--text); }
  .search:focus { border-color: var(--accent); outline: none; }
  .list { margin-top: 8px; max-height: 44vh; overflow-y: auto; border-radius: var(--r-lg); background: var(--fill); }
  .row { display: grid; grid-template-columns: 36px 1fr auto; align-items: center; width: 100%; height: 34px; padding: 0 12px; text-align: left; }
  .row:hover { background: var(--fill); }
  .row.sel { background: var(--accent); color: var(--on-accent); }
  .n { font-variant-numeric: tabular-nums; color: var(--text-3); }
  .row.sel .n { color: inherit; }
  .ar { font-size: 18px; }
  .opts { display: flex; flex-wrap: wrap; align-items: center; gap: 16px; margin: 14px 0 4px; }
  .opts select { margin-left: 6px; padding: 3px 8px; border-radius: var(--r-sm); border: 1px solid var(--sep); background: var(--bg); color: var(--text); }
  .check { display: flex; align-items: center; gap: 6px; }
  .actions { display: flex; gap: 8px; margin-top: 14px; }
  .sp { flex: 1; }
  .btn:disabled { opacity: 0.5; }
</style>
