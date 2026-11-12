<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog'
  import FolderSimplePlus from 'phosphor-svelte/lib/FolderSimplePlus'
  import Waveform from 'phosphor-svelte/lib/Waveform'
  import { addFolder } from '../lib/library.svelte'

  async function pick() {
    const dir = await open({ directory: true, multiple: false, title: 'Choose your music folder' })
    if (typeof dir === 'string') await addFolder(dir)
  }
</script>

<div class="welcome">
  <span class="mark"><Waveform size={44} weight="bold" /></span>
  <h1 class="t-title">Your music, bit for bit.</h1>
  <p class="t-sub">Add the folder where your music and videos live. Sonora plays FLAC, ALAC, WAV, AIFF, APE, WavPack, MP3, AAC, Opus and nearly every video format, and sends lossless audio to your DAC bit-perfect, in exclusive mode.</p>
  <button class="btn primary press" onclick={pick}><FolderSimplePlus size={16} />Add Music Folder…</button>
</div>

<style>
  .welcome { max-width: 460px; margin: 12vh auto 0; text-align: center; display: grid; justify-items: center; gap: 14px; }
  .mark { width: 84px; height: 84px; border-radius: 22px; display: grid; place-items: center; color: #fff; background: linear-gradient(160deg, #ff6b81, var(--accent) 60%, #c81e4a); box-shadow: var(--shadow-art); }
  h1, p { margin: 0; }
  p { font-size: 13px; line-height: 1.55; }
  .btn { margin-top: 8px; height: 34px; padding: 0 18px; }
</style>
