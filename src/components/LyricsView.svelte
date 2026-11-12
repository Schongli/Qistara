<script lang="ts">
  import { Spring } from 'svelte/motion'
  import { mpv, seek } from '../lib/player.svelte'
  import { lyr } from '../lib/lyrics.svelte'
  import { targetLanguage, tr } from '../lib/translate.svelte'
  import { card, learn, openWord } from '../lib/learn.svelte'
  import { settings } from '../lib/settings.svelte'
  import { furigana, toHiragana } from '../lib/kana'
  import { activeLine, isGap, type Line, type Word } from '../lib/lyrics'
  import type { Piece } from '../lib/japanese'

  const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches

  // mpv reports time a few times a second; interpolate per frame so the word sweep is smooth
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

  const data = $derived(lyr.data)
  const lines = $derived(data?.lines ?? [])
  const idx = $derived(data?.synced ? activeLine(lines, now + 0.15) : -1)
  const intro = $derived(data?.synced && lines.length > 0 && isGap(undefined, lines[0]) && idx < 0)
  const learning = $derived(settings.learnLyrics && !!learn.pieces)

  let h = $state(600)
  const els: HTMLElement[] = $state([])
  const y = new Spring(0, { stiffness: 0.07, damping: 0.85 })
  let manual = $state(false)
  let timer: ReturnType<typeof setTimeout>

  // keep the sung line a third of the way down, like Apple Music
  $effect(() => {
    // an open word card holds the lines still, so the word you're reading doesn't move away
    if (manual || card.open || !data?.synced) return
    const el = els[Math.max(0, idx)]
    if (!el) return
    const target = el.offsetTop - h * 0.32
    if (reduced) y.set(target, { instant: true })
    else y.target = target
  })

  function wheel(e: WheelEvent) {
    if (!data?.synced) return
    e.preventDefault()
    manual = true
    y.set(y.current + e.deltaY, { instant: true })
    clearTimeout(timer)
    timer = setTimeout(() => (manual = false), 3000)
  }

  const fill = (w: Word) => Math.min(100, Math.max(0, ((now - w.t) / Math.max(0.05, w.end - w.t)) * 100))
  // three dots fill one by one across an instrumental break
  const dots = (start: number, end: number) => Math.min(1, Math.max(0, (now - start) / Math.max(0.1, end - start)))
  const breakOf = (l: Line, i: number) => ({ start: l.t, end: lines[i + 1]?.t ?? l.end })
</script>

<!-- the translation sits under its line, like Apple Music; the word sweep stays on the original -->
{#snippet translation(i: number)}
  {#if tr.lines?.[i]}<span class="tr" lang={targetLanguage()}>{tr.lines[i]}</span>{/if}
{/snippet}

<!-- Learn mode: every word is a button; Japanese gets furigana over kanji and a romaji line -->
{#snippet ruby(p: Piece)}
  {#if p.jp?.reading}
    {#each furigana(p.text, toHiragana(p.jp.reading)) as s, k (k)}{#if s.rt}<ruby>{s.text}<rt>{s.rt}</rt></ruby>{:else}{s.text}{/if}{/each}
  {:else}{p.text}{/if}
{/snippet}

{#snippet learnText(i: number)}
  <span class="words" lang={learn.japanese ? 'ja' : undefined}>
    {#each learn.pieces?.[i] ?? [] as p, j (j)}
      {#if p.tap}
        <button class="wd" class:sel={card.open && card.line === i && card.piece === j} onclick={() => openWord(i, j)}>{@render ruby(p)}</button>
      {:else}{p.text}{/if}
    {/each}
  </span>
  {#if learn.romaji?.[i]}<span class="ro">{learn.romaji[i]}</span>{/if}
{/snippet}

{#snippet dotRow(p: number, on: boolean)}
  <div class="dots" class:on aria-label="Instrumental">
    {#each [0, 1, 2] as d (d)}<i style:opacity={on ? 0.35 + 0.65 * Math.min(1, Math.max(0, p * 3 - d)) : 0.3}></i>{/each}
  </div>
{/snippet}

<div class="lyrics" class:manual class:learning class:plain={data && !data.synced} bind:clientHeight={h} onwheel={wheel}>
  {#if lyr.loading}
    <p class="note">Looking for lyrics…</p>
  {:else if !data}
    <p class="note">No lyrics for this song.</p>
  {:else if !data.synced}
    <div class="static">
      {#each lines as l, i (i)}<p>{#if learning}{@render learnText(i)}{:else}{l.text}{/if}{@render translation(i)}</p>{/each}
    </div>
  {:else}
    <div class="inner" style:transform="translateY({-y.current}px)">
      {#if intro}{@render dotRow(dots(0, lines[0].t), true)}{/if}
      {#each lines as l, i (i)}
        {@const d = Math.min(4, Math.abs(i - Math.max(0, idx)))}
        {#if !l.text}
          <div bind:this={els[i]}>{@render dotRow(dots(breakOf(l, i).start, breakOf(l, i).end), i === idx)}</div>
        {:else if learning}
          <!-- a div, not a button: the words inside are the buttons (the word card plays the line) -->
          <div bind:this={els[i]} class="line" class:on={i === idx} class:past={i < idx}>
            {@render learnText(i)}
            {@render translation(i)}
          </div>
        {:else}
          <button bind:this={els[i]} class="line" class:on={i === idx} class:past={i < idx} style:--d={manual ? 0 : d} onclick={() => seek(l.t)}>
            {#if l.words && i === idx}
              {#each l.words as w, j (j)}<span class="w" style:--p="{fill(w)}%">{w.text}</span>{/each}
            {:else}
              {l.text}
            {/if}
            {@render translation(i)}
          </button>
          {#if i === idx && isGap(l, lines[i + 1])}{@render dotRow(dots(l.end, lines[i + 1].t), now > l.end)}{/if}
        {/if}
      {/each}
      <div style:height="{h * 0.6}px"></div>
    </div>
  {/if}
  {#if data && lyr.source === 'lrclib'}<p class="credit">Lyrics from LRCLIB</p>{/if}
</div>

<style>
  .lyrics {
    position: relative; height: 100%; overflow: hidden;
    mask-image: linear-gradient(transparent, #000 12%, #000 80%, transparent);
  }
  .lyrics.plain { overflow-y: auto; mask-image: none; }
  .inner { will-change: transform; padding: 0 8px; }
  .line {
    display: block; width: 100%; text-align: left; padding: 10px 12px; margin: 2px 0; border-radius: 12px;
    font-size: clamp(24px, 2.4vw, 42px); line-height: 1.22; font-weight: 750; letter-spacing: -0.022em;
    color: rgba(255, 255, 255, 0.34); filter: blur(calc(var(--d) * 0.55px));
    transform: scale(0.97); transform-origin: left center;
    transition: color 380ms var(--ease-out), filter 380ms var(--ease-out), transform 480ms cubic-bezier(0.25, 1.2, 0.4, 1);
  }
  .line:hover { background: rgba(255, 255, 255, 0.07); filter: none; }
  .line.on { color: #fff; filter: none; transform: none; }
  .manual .line { filter: none; color: rgba(255, 255, 255, 0.55); }
  .manual .line.on { color: #fff; }
  .w {
    background: linear-gradient(90deg, #fff var(--p), rgba(255, 255, 255, 0.36) var(--p));
    -webkit-background-clip: text; background-clip: text; color: transparent;
  }
  /* smaller and a step dimmer than its line, but it follows the line's colour and blur */
  .tr { display: block; margin-top: 0.3em; font-size: 0.56em; font-weight: 600; line-height: 1.3; letter-spacing: -0.01em; opacity: 0.72; }
  .static .tr { margin-top: 2px; font-size: 0.72em; }
  /* Learn mode: smaller, unblurred lines so furigana and romaji stay readable */
  .learning .line { font-size: clamp(20px, 1.8vw, 30px); filter: none; cursor: default; }
  .learning .line:not(.on) { color: rgba(255, 255, 255, 0.5); }
  .learning .line:hover { background: none; }
  .words { display: block; }
  .wd { font: inherit; color: inherit; letter-spacing: inherit; padding: 0 2px; margin: 0 -1px; border-radius: 6px; cursor: pointer; transition: background 120ms; }
  .wd:hover { background: rgba(255, 255, 255, 0.12); }
  .wd.sel { background: rgba(255, 255, 255, 0.26); color: #fff; }
  rt { font-size: 0.42em; font-weight: 600; letter-spacing: 0; opacity: 0.85; }
  .ro { display: block; margin-top: 0.25em; font-size: 0.5em; font-weight: 500; letter-spacing: 0; opacity: 0.7; }
  .dots { display: flex; gap: 10px; padding: 18px 14px; }
  .dots i { width: 12px; height: 12px; border-radius: 50%; background: #fff; transition: opacity 200ms; }
  .dots.on { animation: breathe 2.4s ease-in-out infinite; transform-origin: left center; }
  @keyframes breathe { 50% { transform: scale(1.12); } }
  .static p { margin: 0; padding: 4px 12px; font-size: 22px; font-weight: 650; line-height: 1.35; color: rgba(255, 255, 255, 0.8); user-select: text; }
  .note { color: rgba(255, 255, 255, 0.5); font-size: 15px; text-align: center; margin-top: 40%; }
  .credit { position: absolute; bottom: 4px; right: 12px; margin: 0; font-size: 10.5px; color: rgba(255, 255, 255, 0.35); }
  @media (prefers-reduced-motion: reduce) {
    .line { filter: none; transform: none; transition: color 150ms; }
    .dots.on { animation: none; }
  }
</style>
