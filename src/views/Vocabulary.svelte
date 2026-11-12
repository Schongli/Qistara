<script lang="ts">
  // Words saved from Learn mode in the lyrics, with the line each came from and a flashcard review.
  import Play from 'phosphor-svelte/lib/Play'
  import X from 'phosphor-svelte/lib/X'
  import Cards from 'phosphor-svelte/lib/Cards'
  import { dueItems, type VocabItem } from '../lib/vocab'
  import { grade, playLine, removeWord, vocab } from '../lib/learn.svelte'
  import { furigana } from '../lib/kana'
  import { plural } from '../lib/format'

  const due = $derived(dueItems(vocab.items))
  // ids still to go in this review; a missed word goes to the back and comes round again
  let queue = $state<string[] | null>(null)
  let revealed = $state(false)
  const current = $derived(queue?.length ? vocab.items.find((i) => i.id === queue![0]) : undefined)

  function start() {
    queue = due.map((i) => i.id)
    revealed = false
  }
  function answer(knew: boolean) {
    if (!current || !queue) return
    grade(current.id, knew)
    const [first, ...rest] = queue
    queue = knew ? rest : [...rest, first]
    revealed = false
  }
  function keys(e: KeyboardEvent) {
    if (!current || (e.target as HTMLElement).closest('input, textarea, select')) return
    if (e.key === 'Enter' && !revealed) revealed = true
    else if (revealed && e.key === '1') answer(false)
    else if (revealed && e.key === '2') answer(true)
    else return
    e.preventDefault()
  }
  const lang = (i: VocabItem) => (i.lang === 'auto' ? undefined : i.lang)
</script>

<svelte:window onkeydown={keys} />

{#snippet word(i: VocabItem, withReading: boolean)}
  <span class="w" lang={lang(i)}>{#each furigana(i.word, withReading ? i.reading : '') as s, k (k)}{#if s.rt}<ruby>{s.text}<rt>{s.rt}</rt></ruby>{:else}{s.text}{/if}{/each}</span>
{/snippet}

{#snippet example(i: VocabItem)}
  {#if i.line}
    <div class="ex">
      <button class="icon-btn press" aria-label="Play this line" title="Play the line in {i.title}" onclick={() => playLine(i.path, i.t)}><Play size={12} weight="fill" /></button>
      <div class="ex-text">
        <span lang={lang(i)}>{i.line}</span>
        {#if i.translation}<span class="t-sub">{i.translation}</span>{/if}
        <span class="t-sub src ellipsis">{i.title}{i.artist ? ` · ${i.artist}` : ''}</span>
      </div>
    </div>
  {/if}
{/snippet}

{#if queue !== null}
  <section class="review">
    {#if current}
      <p class="t-sub">{plural(queue.length, 'card')} left</p>
      <div class="flash">
        {@render word(current, revealed)}
        {#if revealed}
          <p class="reading">{#if current.romaji}{current.romaji}{/if}{#if current.pos} <span class="pos">{current.pos}</span>{/if}</p>
          <p class="mean">{current.meanings.join(', ')}</p>
          {@render example(current)}
        {/if}
      </div>
      <div class="actions">
        {#if !revealed}
          <button class="btn primary press" onclick={() => (revealed = true)}>Show Answer <kbd>Enter</kbd></button>
        {:else}
          <button class="btn press" onclick={() => answer(false)}>Again <kbd>1</kbd></button>
          <button class="btn primary press" onclick={() => answer(true)}>Got It <kbd>2</kbd></button>
        {/if}
        <button class="btn press end" onclick={() => (queue = null)}>End Review</button>
      </div>
    {:else}
      <div class="flash done">
        <p class="w">All caught up</p>
        <p class="t-sub">Words you know come back after longer and longer breaks: 1, 3, 7, 14, then 30 days.</p>
      </div>
      <div class="actions"><button class="btn primary press" onclick={() => (queue = null)}>Done</button></div>
    {/if}
  </section>
{:else}
  <h1 class="t-title">Vocabulary</h1>
  <div class="bar">
    <p class="t-sub">
      {#if vocab.items.length}{plural(vocab.items.length, 'word')} saved · {due.length} to review{:else}No saved words yet.{/if}
    </p>
    <button class="btn primary press" disabled={!due.length} onclick={start}><Cards size={15} />Review</button>
  </div>

  {#if !vocab.items.length}
    <p class="empty t-sub">
      In Now Playing, open the lyrics and turn on Learn (the graduation cap). Tap any word to see its reading and meaning, then
      choose Save Word. Saved words appear here with the line they came from.
    </p>
  {/if}

  {#each vocab.items as i (i.id)}
    <div class="row">
      <div class="head">
        {@render word(i, true)}
        {#if i.romaji}<span class="t-sub">{i.romaji}</span>{/if}
        {#if i.pos}<span class="pos">{i.pos}</span>{/if}
        <button class="icon-btn press rm" aria-label="Remove {i.word}" title="Remove" onclick={() => removeWord(i.id)}><X size={13} weight="bold" /></button>
      </div>
      <p class="mean">{i.meanings.join(', ')}</p>
      {@render example(i)}
    </div>
  {/each}
{/if}

<style>
  h1 { margin: 0 0 6px; }
  .bar { display: flex; align-items: center; justify-content: space-between; gap: 12px; max-width: 820px; margin-bottom: 18px; }
  .bar p { margin: 0; }
  .empty { max-width: 560px; line-height: 1.55; }
  .row { max-width: 820px; padding: 12px 14px; margin-bottom: 10px; border-radius: 12px; background: var(--fill); }
  .head { display: flex; align-items: baseline; gap: 10px; }
  .w { font-size: 22px; font-weight: 700; letter-spacing: -0.015em; line-height: 1.3; }
  rt { font-size: 0.45em; font-weight: 600; letter-spacing: 0; color: var(--text-2); }
  .pos { font-size: 10.5px; font-weight: 650; text-transform: uppercase; letter-spacing: 0.05em; color: var(--text-3); }
  .rm { margin-left: auto; color: var(--text-3); }
  .mean { margin: 4px 0 0; line-height: 1.4; }
  .ex { display: flex; gap: 8px; align-items: flex-start; margin-top: 8px; padding-top: 8px; border-top: 1px solid var(--sep); }
  .ex-text { display: grid; gap: 1px; min-width: 0; font-size: 13.5px; line-height: 1.4; }
  .src { font-size: 12px; }
  .review { max-width: 620px; margin: 40px auto 0; text-align: center; }
  .flash { padding: 36px 28px; border-radius: 18px; background: var(--fill); }
  .flash .w { font-size: 46px; }
  .flash .ex { text-align: left; margin-top: 18px; }
  .reading { margin: 8px 0 0; font-size: 17px; color: var(--text-2); }
  .flash .mean { font-size: 17px; margin-top: 10px; }
  .done .w { font-size: 26px; margin: 0 0 6px; }
  .actions { display: flex; gap: 10px; justify-content: center; margin-top: 18px; }
  .end { margin-left: 18px; }
  kbd { margin-left: 6px; padding: 0 5px; border-radius: 4px; font: 600 10.5px/16px inherit; background: rgba(127, 127, 127, 0.2); }
</style>
