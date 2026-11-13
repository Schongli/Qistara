<script lang="ts">
  // The lyrics pane for a Qur'an recitation: each ayah in Uthmani script with transliteration and
  // translations, the ayah being recited kept in view, and a card for any word you tap.
  import { Spring } from 'svelte/motion'
  import { fly } from 'svelte/transition'
  import { cubicOut } from 'svelte/easing'
  import X from 'phosphor-svelte/lib/X'
  import Star from 'phosphor-svelte/lib/Star'
  import Play from 'phosphor-svelte/lib/Play'
  import Crosshair from 'phosphor-svelte/lib/Crosshair'
  import ArrowCounterClockwise from 'phosphor-svelte/lib/ArrowCounterClockwise'
  import { mpv, seek, set } from '../lib/player.svelte'
  import { queue } from '../lib/queue.svelte'
  import { settings } from '../lib/settings.svelte'
  import { BASMALA, ISTIADHA, TRANSLATIONS, clearMarks, currentUnit, markStart, quran, type QAyah, type QWord } from '../lib/quran.svelte'
  import { SURAHS } from '../lib/quran'
  import { isSaved, removeWord, saveWord, vocab } from '../lib/learn.svelte'
  import { vocabId } from '../lib/vocab'

  const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches

  // mpv reports time a few times a second; interpolate per frame so highlights move smoothly
  let now = $state(0)
  let base = { t: 0, at: 0 }
  $effect(() => {
    base = { t: mpv.time, at: performance.now() }
  })
  $effect(() => {
    let raf = 0
    const tick = () => {
      now = mpv.pause ? mpv.time : base.t + ((performance.now() - base.at) / 1000) * mpv.speed
      raf = requestAnimationFrame(tick)
    }
    tick()
    return () => cancelAnimationFrame(raf)
  })
  const idx = $derived(currentUnit(now + 0.1))

  // keep the ayah being recited a little above the middle; scrolling by hand pauses that for 3 s
  let h = $state(600)
  const els: HTMLElement[] = $state([])
  const y = new Spring(0, { stiffness: 0.07, damping: 0.85 })
  let manual = $state(false)
  let timer: ReturnType<typeof setTimeout>
  $effect(() => {
    if (manual || idx < 0) return
    const el = els[idx]
    if (!el) return
    const target = Math.max(0, el.offsetTop - h * 0.28)
    if (reduced) y.set(target, { instant: true })
    else y.target = target
  })
  function wheel(e: WheelEvent) {
    e.preventDefault()
    manual = true
    y.set(Math.max(0, y.current + e.deltaY), { instant: true })
    clearTimeout(timer)
    timer = setTimeout(() => (manual = false), 3000)
  }

  const rec = $derived(quran.rec)
  const surahs = $derived(rec?.surahs.map((n) => SURAHS[n - 1]) ?? [])
  const title = $derived(
    !rec ? '' : rec.surahs.length > 1 ? (rec.surahs[0] === 78 ? "Juz 'Amma" : 'Juz Tabarak') : `${surahs[0].en}`,
  )
  const riwayah = $derived(rec?.riwayah === 'shubah' ? "Shu'bah 'an 'Asim" : "Hafs 'an 'Asim")
  const shown = $derived(TRANSLATIONS.filter((t) => settings.quranTranslations.includes(t.id)))
  const toggleTr = (id: string) =>
    (settings.quranTranslations = settings.quranTranslations.includes(id) ? settings.quranTranslations.filter((x) => x !== id) : [...settings.quranTranslations, id])
  const translit = (a: QAyah) => a.words?.map((w) => w.tr).filter(Boolean).join(' ') ?? ''
  const arNum = (n: number) => n.toLocaleString('ar-EG')

  async function go(t: number | undefined) {
    if (t === undefined) return
    await seek(t)
    if (mpv.pause) await set('pause', false)
  }
  const startOf = (i: number) => quran.starts?.[i] ?? quran.anchors[i]
  function tapUnit(i: number) {
    if (quran.fixing) markStart(i, mpv.time)
    else go(startOf(i))
  }

  // ---- the word card ----
  let card = $state<{ unit: number; ayah: QAyah; word: QWord } | null>(null)
  function tapWord(e: MouseEvent, unit: number, ayah: QAyah, word: QWord) {
    e.stopPropagation()
    if (quran.fixing) return markStart(unit, mpv.time)
    card = { unit, ayah, word }
  }
  const cardId = $derived(card ? vocabId('ar', card.word.ar) : '')
  const saved = $derived(!!cardId && vocab.loaded && isSaved(cardId))
  function toggleSave() {
    if (!card) return
    if (saved) return removeWord(cardId)
    const { ayah, word, unit } = card
    const t = queue.current
    saveWord({
      id: cardId, word: word.ar, reading: '', romaji: word.tr, pos: '', meanings: [word.mean].filter(Boolean), lang: 'ar',
      line: ayah.ar, translation: shown.map((s) => ayah.tr[s.id]).find(Boolean) ?? '',
      path: t?.path ?? '', title: `${SURAHS[ayah.surah - 1].en} ${ayah.surah}:${ayah.n}`, artist: t?.artist ?? '', t: word.seg?.[0] ?? startOf(unit) ?? 0,
    })
  }
  function keys(e: KeyboardEvent) {
    if (e.key !== 'Escape' || (!card && !quran.fixing)) return
    e.stopPropagation()
    card = null
    quran.fixing = false
  }
</script>

<svelte:window onkeydown={keys} />

<div class="quran" bind:clientHeight={h} onwheel={wheel}>
  <header>
    <div class="title">
      <span class="name" lang="ar">{rec && rec.surahs.length === 1 ? `سورة ${surahs[0].ar}` : rec?.surahs[0] === 78 ? 'جزء عمّ' : 'جزء تبارك'}</span>
      <span class="sub">{title} · {riwayah}</span>
    </div>
    <div class="tools">
      {#each TRANSLATIONS as t (t.id)}
        <button class="chip press" class:on={settings.quranTranslations.includes(t.id)} aria-pressed={settings.quranTranslations.includes(t.id)} title={t.name} onclick={() => toggleTr(t.id)}>{t.short}</button>
      {/each}
      <button class="chip press" class:on={settings.quranTranslit} aria-pressed={settings.quranTranslit} title="Transliteration" onclick={() => (settings.quranTranslit = !settings.quranTranslit)}>Latin</button>
      <button class="chip press fix" class:on={quran.fixing} aria-pressed={quran.fixing} title="Tap the ayah being recited to sync it" onclick={() => (quran.fixing = !quran.fixing)}><Crosshair size={13} weight="bold" />Fix sync</button>
      {#if Object.keys(quran.anchors).length}
        <button class="chip press" title="Forget the ayahs you tapped" onclick={clearMarks}><ArrowCounterClockwise size={13} weight="bold" /></button>
      {/if}
    </div>
    {#if quran.fixing}
      <p class="note on">Tap the ayah being recited right now. Esc cancels.</p>
    {:else if quran.status === 'loading'}
      <p class="note">Loading the surah…</p>
    {:else if quran.status === 'analysing'}
      <p class="note">Finding the ayahs in this recording (once per surah)…</p>
    {:else if quran.status === 'error'}
      <p class="note err">{quran.error}</p>
    {:else if (rec?.partial || quran.stream) && !Object.keys(quran.anchors).length}
      <p class="note">{quran.stream ? "A stream can't be scanned for the pauses between ayahs" : "This is a selection of ayahs, so it can't be synced on its own"}: use Fix sync on the ayah being recited.</p>
    {:else if rec?.riwayah === 'shubah'}
      <p class="note">Shu'bah text from the King Fahd Complex; word meanings follow Hafs where the words match.</p>
    {/if}
  </header>

  <div class="scroll" class:manual>
    <div class="inner" style:transform="translateY({-y.current}px)">
      {#each quran.units as u, i (i)}
        {@const d = Math.min(3, Math.abs(i - Math.max(0, idx)))}
        {#if u.kind === 'istiadha'}
          <button bind:this={els[i]} class="unit opening press" class:on={i === idx} style:--d={d} onclick={() => tapUnit(i)}>
            <span lang="ar" dir="rtl">{ISTIADHA}</span>
          </button>
        {:else if u.kind === 'basmala'}
          <button bind:this={els[i]} class="unit basmala press" class:on={i === idx} style:--d={d} onclick={() => tapUnit(i)}>
            {#if rec && rec.surahs.length > 1}<span class="surah"><span lang="ar">سورة {SURAHS[u.surah - 1].ar}</span> · {SURAHS[u.surah - 1].en}</span>{/if}
            <span lang="ar" dir="rtl">{BASMALA}</span>
          </button>
        {:else}
          {@const a = u.ayah}
          <div bind:this={els[i]} class="unit ayah" class:on={i === idx} class:past={i < idx} style:--d={d} role="button" tabindex="0" onclick={() => tapUnit(i)} onkeydown={(e) => e.key === 'Enter' && tapUnit(i)}>
            <p class="ar" lang="ar" dir="rtl">
              {#if a.words}
                {#each a.words as w, k (k)}
                  <button class="w" class:now={quran.exact && i === idx && w.seg && now >= w.seg[0] && now < w.seg[1]} onclick={(e) => tapWord(e, i, a, w)}>{w.ar}</button>{' '}
                {/each}
              {:else}
                {a.ar}
              {/if}
              <span class="num" aria-label="Ayah {a.n}">{arNum(a.n)}</span>
            </p>
            {#if settings.quranTranslit && a.words}<p class="tr">{translit(a)}</p>{/if}
            {#each shown as t (t.id)}
              {#if a.tr[t.id]}<p class="mean" lang={t.lang}>{a.tr[t.id]}{#if shown.length > 1}<span class="src"> — {t.short}</span>{/if}</p>{/if}
            {/each}
          </div>
        {/if}
      {/each}
      <div style:height="{h * 0.6}px"></div>
    </div>
  </div>

  {#if card}
    <div class="card" role="dialog" aria-label="Word" transition:fly={{ y: reduced ? 0 : 24, duration: 220, easing: cubicOut }}>
      <button class="icon-btn press close" aria-label="Close" onclick={() => (card = null)}><X size={15} weight="bold" /></button>
      <div class="head" lang="ar" dir="rtl">{card.word.ar}</div>
      {#if card.word.tr}<div class="say">{card.word.tr}</div>{/if}
      <div class="meaning">{card.word.mean || '—'}</div>
      <div class="from">{SURAHS[card.ayah.surah - 1].en} {card.ayah.surah}:{card.ayah.n}</div>
      <div class="actions">
        <button class="btn press" onclick={() => card && go(card.word.seg?.[0] ?? startOf(card.unit))}><Play size={13} weight="fill" />{card.word.seg ? 'Hear it' : 'Hear the ayah'}</button>
        <button class="btn press" class:primary={!saved} onclick={toggleSave}><Star size={13} weight={saved ? 'fill' : 'bold'} />{saved ? 'Saved' : 'Save to Vocabulary'}</button>
      </div>
    </div>
  {/if}
</div>

<style>
  .quran { position: relative; height: 100%; display: flex; flex-direction: column; color: #fff; }
  header { flex: none; padding: 10px 12px 8px; display: grid; gap: 6px; }
  .title { display: flex; align-items: baseline; gap: 10px; min-width: 0; }
  .name { font-size: 22px; }
  .sub { font-size: 12px; color: rgba(255, 255, 255, 0.6); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .tools { display: flex; flex-wrap: wrap; gap: 6px; }
  .chip { display: inline-flex; align-items: center; gap: 4px; height: 24px; padding: 0 10px; border-radius: 12px; font-size: 12px; color: rgba(255, 255, 255, 0.75); background: rgba(255, 255, 255, 0.1); }
  .chip:hover { background: rgba(255, 255, 255, 0.16); }
  .chip.on { color: #111; background: rgba(255, 255, 255, 0.9); }
  .note { margin: 0; font-size: 12px; color: rgba(255, 255, 255, 0.6); line-height: 1.4; }
  .note.on { color: #fff; }
  .note.err { color: #ff8a8a; }

  .scroll { position: relative; flex: 1; overflow: hidden; mask-image: linear-gradient(transparent, #000 8%, #000 82%, transparent); }
  .inner { will-change: transform; padding: 0 8px; }
  .unit { display: block; width: 100%; text-align: center; padding: 10px 12px; margin: 4px 0; border-radius: 14px; color: rgba(255, 255, 255, 0.42); filter: blur(calc(var(--d) * 0.4px)); transition: color 380ms var(--ease-out), filter 380ms var(--ease-out), background 380ms; }
  .manual .unit { filter: none; color: rgba(255, 255, 255, 0.7); }
  .unit.on { color: #fff; filter: none; background: rgba(255, 255, 255, 0.06); }
  .unit:hover { filter: none; }
  .opening, .basmala { font-size: calc(22px * var(--lyrics-scale, 1)); line-height: 2; display: grid; gap: 2px; }
  .surah { display: block; font-size: 12px; letter-spacing: 0.02em; color: rgba(255, 255, 255, 0.55); margin-top: 10px; }
  .ayah { cursor: default; }
  .ar { margin: 0; font-size: calc(30px * var(--lyrics-scale, 1)); line-height: 2.15; }
  .w { font: inherit; color: inherit; padding: 0 1px; border-radius: 6px; }
  .w:hover { background: rgba(255, 255, 255, 0.12); }
  .w.now { color: #fff; text-shadow: 0 0 14px rgba(255, 255, 255, 0.55); }
  .ayah:not(.on) .w.now { text-shadow: none; }
  .num { display: inline-grid; place-items: center; min-width: 1.6em; height: 1.6em; margin-inline-start: 4px; border: 1.5px solid currentColor; border-radius: 50%; font-size: 0.5em; line-height: 1; vertical-align: middle; opacity: 0.7; }
  .tr { margin: 2px 0 0; font-size: calc(14px * var(--lyrics-scale, 1)); font-style: italic; line-height: 1.45; opacity: 0.8; }
  .mean { margin: 6px 0 0; font-size: calc(15px * var(--lyrics-scale, 1)); line-height: 1.45; opacity: 0.85; text-align: start; }
  .src { font-size: 11px; opacity: 0.6; }

  .card {
    position: absolute; left: 10px; right: 10px; bottom: 10px; padding: 16px 18px 14px; border-radius: 16px; color: #fff;
    background: rgba(28, 28, 30, 0.78); backdrop-filter: blur(30px) saturate(1.6); box-shadow: 0 18px 48px rgba(0, 0, 0, 0.45);
  }
  .close { position: absolute; top: 8px; right: 8px; color: rgba(255, 255, 255, 0.7); }
  .head { font-size: 40px; line-height: 1.7; text-align: center; }
  .say { text-align: center; font-size: 15px; font-style: italic; color: rgba(255, 255, 255, 0.75); }
  .meaning { margin-top: 6px; text-align: center; font-size: 17px; font-weight: 600; }
  .from { margin-top: 2px; text-align: center; font-size: 12px; color: rgba(255, 255, 255, 0.5); }
  .actions { display: flex; justify-content: center; gap: 8px; margin-top: 12px; }
  @media (prefers-reduced-motion: reduce) {
    .unit { filter: none; transition: color 150ms; }
  }
</style>
