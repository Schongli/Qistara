<script lang="ts">
  // The card that slides up over the lyrics when you tap a word in Learn mode.
  import { fly } from 'svelte/transition'
  import { cubicOut } from 'svelte/easing'
  import X from 'phosphor-svelte/lib/X'
  import Star from 'phosphor-svelte/lib/Star'
  import Play from 'phosphor-svelte/lib/Play'
  import { card, cardId, closeCard, isSaved, learn, toggleSaveCard, vocab } from '../lib/learn.svelte'
  import { lyr } from '../lib/lyrics.svelte'
  import { tr } from '../lib/translate.svelte'
  import { mpv, seek, set } from '../lib/player.svelte'
  import { furigana } from '../lib/kana'

  const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches
  const line = $derived(lyr.data?.lines[card.line])
  const saved = $derived(isSaved(cardId()))
  const head = $derived(furigana(card.text, card.sung))
  const lang = $derived(learn.japanese ? 'ja' : undefined)

  async function playLine() {
    if (!line) return
    await seek(line.t)
    if (mpv.pause) await set('pause', false)
  }

  // Escape closes the card first, before the app's Escape closes Now Playing
  function keys(e: KeyboardEvent) {
    if (e.key !== 'Escape') return
    e.stopPropagation()
    closeCard()
  }
</script>

<svelte:window onkeydowncapture={keys} />

<div class="card" role="dialog" aria-label="Word: {card.word}" transition:fly={{ y: 28, duration: reduced ? 0 : 260, easing: cubicOut }}>
  <button class="icon-btn close" aria-label="Close" onclick={closeCard}><X size={14} weight="bold" /></button>

  <div class="head">
    <span class="word" {lang}>{#each head as s, k (k)}{#if s.rt}<ruby>{s.text}<rt>{s.rt}</rt></ruby>{:else}{s.text}{/if}{/each}</span>
    {#if card.pos}<span class="pos">{card.pos}</span>{/if}
  </div>

  {#if card.word !== card.text || card.romaji}
    <p class="form">
      {#if card.word !== card.text}<span class="label">Dictionary form</span> <span {lang}>{card.word}</span>{/if}
      {#if card.reading && card.reading !== card.word}<span {lang}>{card.reading}</span>{/if}
      {#if card.romaji}<i>{card.romaji}</i>{/if}
    </p>
  {/if}

  <div class="meaning">
    {#if card.note}
      <p>{card.note}</p>
    {:else if card.busy}
      <p class="dim">Looking up…</p>
    {:else if card.error}
      <p class="dim">{card.error}</p>
    {:else if card.lookup?.senses.length}
      {#each card.lookup.senses.slice(0, 3) as s, k (k)}<p><span class="pos">{s.pos}</span> {s.terms.slice(0, 5).join(', ')}</p>{/each}
    {:else if card.lookup?.translation}
      <p>{card.lookup.translation}</p>
    {/if}
  </div>

  {#if line}
    <div class="ctx">
      <p {lang}>{#each learn.pieces?.[card.line] ?? [] as p, j (j)}{#if j === card.piece}<b>{p.text}</b>{:else}{p.text}{/if}{/each}</p>
      {#if tr.lines?.[card.line]}<p class="dim">{tr.lines[card.line]}</p>{/if}
    </div>
  {/if}

  <div class="actions">
    {#if line && lyr.data?.synced}<button class="btn press" onclick={playLine}><Play size={13} weight="fill" />Play Line</button>{/if}
    <button class="btn press" class:saved disabled={!vocab.loaded} onclick={toggleSaveCard}>
      <Star size={14} weight={saved ? 'fill' : 'regular'} />{saved ? 'Saved' : 'Save Word'}
    </button>
  </div>
</div>

<style>
  .card {
    position: absolute; left: 10px; right: 10px; bottom: 10px; z-index: 3; padding: 16px 18px 14px;
    border-radius: 16px; color: #fff;
    background: rgba(28, 28, 30, 0.72); backdrop-filter: blur(30px) saturate(1.6);
    border: 0.5px solid rgba(255, 255, 255, 0.14); box-shadow: 0 12px 40px rgba(0, 0, 0, 0.35);
  }
  @media (prefers-reduced-transparency: reduce) { .card { background: rgb(38, 38, 40); backdrop-filter: none; } }
  .close { position: absolute; top: 10px; right: 10px; color: rgba(255, 255, 255, 0.6); }
  .head { display: flex; align-items: baseline; gap: 10px; padding-right: 28px; }
  .word { font-size: 30px; font-weight: 700; letter-spacing: -0.02em; line-height: 1.25; }
  rt { font-size: 0.4em; font-weight: 600; letter-spacing: 0; color: rgba(255, 255, 255, 0.75); }
  .pos { font-size: 11px; font-weight: 650; text-transform: uppercase; letter-spacing: 0.05em; color: rgba(255, 255, 255, 0.55); }
  .form { display: flex; flex-wrap: wrap; gap: 8px; align-items: baseline; margin: 4px 0 0; font-size: 15px; color: rgba(255, 255, 255, 0.85); }
  .form i { font-style: normal; color: rgba(255, 255, 255, 0.6); }
  .label { font-size: 11px; color: rgba(255, 255, 255, 0.5); }
  .meaning { margin: 10px 0 0; font-size: 15px; line-height: 1.4; }
  .meaning p { margin: 0 0 3px; }
  .meaning .pos { margin-right: 6px; }
  .ctx { margin-top: 12px; padding-top: 10px; border-top: 0.5px solid rgba(255, 255, 255, 0.14); font-size: 13.5px; line-height: 1.4; }
  .ctx p { margin: 0 0 2px; }
  .ctx b { color: #fff; font-weight: 700; }
  .dim { color: rgba(255, 255, 255, 0.55); }
  .actions { display: flex; gap: 8px; margin-top: 12px; }
  .actions .btn { color: #fff; background: rgba(255, 255, 255, 0.14); }
  .actions .btn:hover { background: rgba(255, 255, 255, 0.22); }
  .actions .saved { color: #ffd60a; }
</style>
