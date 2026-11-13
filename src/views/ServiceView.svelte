<script lang="ts">
  // Apple Music or Tidal: your playlists, imported as playlists of your own files.
  import { untrack } from 'svelte'
  import ImportList from '../components/ImportList.svelte'
  import { ALL, services, type ServiceId } from '../lib/services.svelte'
  import { nav } from '../lib/nav.svelte'

  let { kind }: { kind: ServiceId } = $props()
  const svc = $derived(services[kind])
  const s = $derived(svc.s)
  $effect(() => {
    if (s.user) untrack(() => !s.playlists.length && !s.loading && svc.load())
  })
  const rows = $derived([{ id: ALL, name: s.allLabel, tracks: 0, image: null, all: true }, ...s.playlists])
</script>

<h1 class="t-title">{s.label}</h1>
{#if !s.user}
  <p class="t-sub">Sign in to {s.label} in Providers to import your playlists, matched to the files you have.</p>
  <button class="btn primary press" onclick={() => nav.root({ name: 'providers' })}>Open Providers</button>
{:else}
  <p class="t-sub">Signed in as {s.user}. {s.label} streams can't play here (they're DRM-protected); your own copies do, in full quality.</p>
  <ImportList rows={rows} results={s.results} importing={s.importing} loading={s.loading} service={s.label} onimport={svc.importOne} onreload={svc.load} />
  {#if s.error}<p class="err">{s.error}</p>{/if}
{/if}

<style>
  h1 { margin: 0 0 4px; }
  p { margin: 0 0 12px; line-height: 1.5; max-width: 720px; }
  .err { color: #e5484d; }
</style>
