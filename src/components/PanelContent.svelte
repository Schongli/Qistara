<script lang="ts">
  // Lyrics (or the Qur'an view for a recitation) or Up Next, white on a dark backdrop. Shared by the
  // Now Playing sheet and the pane docked beside the library.
  import Translate from 'phosphor-svelte/lib/Translate'
  import GraduationCap from 'phosphor-svelte/lib/GraduationCap'
  import WordCard from './WordCard.svelte'
  import QueueList from './QueueList.svelte'
  import LyricsView from './LyricsView.svelte'
  import QuranView from './QuranView.svelte'
  import { card, learn } from '../lib/learn.svelte'
  import { lyr } from '../lib/lyrics.svelte'
  import { targetLanguage, tr } from '../lib/translate.svelte'
  import { languageName } from '../lib/translate'
  import { quran } from '../lib/quran.svelte'
  import { settings } from '../lib/settings.svelte'

  let { mode }: { mode: 'lyrics' | 'queue' } = $props()
</script>

{#if mode === 'queue'}
  <div class="pane queue"><QueueList dark /></div>
{:else if quran.rec}
  <div class="pane"><QuranView /></div>
{:else}
  <div class="pane">
    <LyricsView />
    {#if lyr.data}
      {@const lang = languageName(targetLanguage())}
      <!-- outside LyricsView so its top fade never dims the buttons -->
      <div class="tools">
        <button
          class="icon-btn press"
          class:on={settings.learnLyrics}
          aria-pressed={settings.learnLyrics}
          aria-label="Learn mode"
          title={settings.learnLyrics ? learn.error || 'Learn mode: tap any word for its meaning' : 'Learn: readings, romaji and word meanings'}
          onclick={() => (settings.learnLyrics = !settings.learnLyrics)}
        ><GraduationCap size={18} weight={settings.learnLyrics ? 'bold' : 'regular'} /></button>
        {#if !settings.learnLyrics}
          <button
            class="icon-btn press"
            class:on={settings.translateLyrics}
            class:busy={tr.busy}
            aria-pressed={settings.translateLyrics}
            aria-label="Translate lyrics to {lang}"
            title={!settings.translateLyrics ? `Translate to ${lang}` : tr.error || (tr.same ? `These lyrics are already in ${lang}` : `Translated to ${lang} by Google`)}
            onclick={() => (settings.translateLyrics = !settings.translateLyrics)}
          ><Translate size={18} weight={settings.translateLyrics ? 'bold' : 'regular'} /></button>
        {/if}
      </div>
    {/if}
    {#if card.open}<WordCard />{/if}
  </div>
{/if}

<style>
  .pane { position: relative; height: 100%; overflow: hidden; color: #fff; }
  .pane.queue { overflow-y: auto; padding: 8px; }
  .tools { position: absolute; top: 6px; right: 6px; z-index: 2; display: flex; gap: 2px; }
  .tools .icon-btn { color: rgba(255, 255, 255, 0.72); }
  .tools .icon-btn:hover { background: rgba(255, 255, 255, 0.12); }
  .tools .icon-btn.on { color: #fff; background: rgba(255, 255, 255, 0.16); }
  .tools .busy { animation: pulse 1.2s ease-in-out infinite; }
  @keyframes pulse { 50% { opacity: 0.45; } }
  @media (prefers-reduced-motion: reduce) { .tools .busy { animation: none; } }
</style>
