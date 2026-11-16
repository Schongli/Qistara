<script lang="ts">
  import { KEY_STORE, OS } from '../lib/os'
  // Where music comes from: your folders, YouTube, your own server, and the services whose playlists can be
  // imported. Each one signs in or out here.
  import { open } from '@tauri-apps/plugin-dialog'
  import { openUrl } from '@tauri-apps/plugin-opener'
  import Icon from '../components/Icon.svelte'
  import { server, signIn as serverSignIn, signOut as serverSignOut } from '../lib/subsonic.svelte'
  import { apple, tidal } from '../lib/services.svelte'
  import { lib } from '../lib/library.svelte'
  import { nav } from '../lib/nav.svelte'
  import { settings } from '../lib/settings.svelte'
  import { plural } from '../lib/format'
  import { connect as connectSpotify, disconnect, setClient, spotify } from '../lib/spotify.svelte'

  // embedded: shown as the Providers tab in Settings
  let { embedded = false }: { embedded?: boolean } = $props()
  let openId = $state('')
  const toggle = (id: string) => (openId = openId === id ? '' : id)
  let clientId = $state('')
  let address = $state('')
  let user = $state('')
  let password = $state('')
  let teamId = $state('')
  let keyId = $state('')
  let tidalClient = $state('')
  async function pickKey() {
    const f = await open({ multiple: false, title: 'Choose your MusicKit private key (.p8)', filters: [{ name: 'MusicKit key', extensions: ['p8'] }] })
    if (typeof f === 'string') await apple.setup({ teamId: teamId.trim(), keyId: keyId.trim(), keyPath: f })
  }
  async function connectServer() {
    await serverSignIn(address, user, password)
    if (server.account) password = ''
  }

  const ytStatus = $derived(settings.ytCookies === 'firefox' ? 'Signed in with Firefox' : settings.ytCookies ? 'Signed in with a cookies file' : 'Guest')
  async function pickCookies() {
    const f = await open({ multiple: false, title: 'Choose a cookies.txt exported from your browser', filters: [{ name: 'cookies.txt', extensions: ['txt'] }] })
    if (typeof f === 'string') settings.ytCookies = f
  }
</script>

{#if !embedded}<h1 class="t-title">Providers</h1>{/if}
<p class="t-sub lede">Your own files always play bit-perfect. Your server and YouTube play here too; Spotify, Apple Music and Tidal lend their playlists, matched to the files you have.</p>

<div class="list">
  <section class="prov">
    <button class="top press" aria-expanded={openId === 'local'} onclick={() => toggle('local')}>
      <span class="ic"><Icon name="FolderSimple" size={20} /></span>
      <span class="name">Local music<span class="t-sub">{plural(lib.folders.length, 'folder')} · {plural(lib.tracks.length, 'file')}</span></span>
      <span class="state on">In use</span><span class="caret" class:open={openId === 'local'}><Icon name="CaretDown" size={12} weight="bold" /></span>
    </button>
    {#if openId === 'local'}
      <div class="body">
        <p class="t-sub">Folders are watched, so new and changed files show up on their own.</p>
        <button class="btn press" onclick={() => nav.root({ name: 'settings' })}>Manage Folders in Settings</button>
      </div>
    {/if}
  </section>

  <section class="prov">
    <button class="top press" aria-expanded={openId === 'youtube'} onclick={() => toggle('youtube')}>
      <span class="ic"><Icon name="YoutubeLogo" size={20} /></span>
      <span class="name">YouTube<span class="t-sub">Search, play and download, through yt-dlp</span></span>
      <span class="state" class:on={!!settings.ytCookies}>{ytStatus}</span><span class="caret" class:open={openId === 'youtube'}><Icon name="CaretDown" size={12} weight="bold" /></span>
    </button>
    {#if openId === 'youtube'}
      <div class="body">
        <p class="t-sub">Signing in lets yt-dlp open your Liked Music, age-restricted videos and, with Premium, 256 kbps audio. It reads the sign-in from your browser's cookies on this PC; your password never leaves the browser.{#if OS === 'windows'} Chrome and Edge lock their cookies to themselves on Windows, so use Firefox or a cookies.txt export.{/if}</p>
        <div class="choices" role="radiogroup" aria-label="YouTube sign-in">
          <label class="choice"><input type="radio" name="yt" checked={!settings.ytCookies} onchange={() => (settings.ytCookies = '')} />Guest</label>
          <label class="choice"><input type="radio" name="yt" checked={settings.ytCookies === 'firefox'} onchange={() => (settings.ytCookies = 'firefox')} />Firefox (signed in to YouTube)</label>
          <label class="choice"><input type="radio" name="yt" checked={!!settings.ytCookies && settings.ytCookies !== 'firefox'} onchange={pickCookies} />cookies.txt file{#if settings.ytCookies && settings.ytCookies !== 'firefox'}<span class="t-sub ellipsis path">{settings.ytCookies}</span>{/if}</label>
        </div>
        <button class="btn press" onclick={() => nav.root({ name: 'youtube' })}>Open YouTube</button>
      </div>
    {/if}
  </section>

  <section class="prov">
    <button class="top press" aria-expanded={openId === 'server'} onclick={() => toggle('server')}>
      <span class="ic"><Icon name="HardDrives" size={20} /></span>
      <span class="name">Navidrome / Subsonic<span class="t-sub">Stream your own music server, lossless included</span></span>
      <span class="state" class:on={!!server.account}>{server.account ? `${server.account.user} on ${server.account.server.replace(/^https?:\/\//, '')}` : 'Sign in'}</span><span class="caret" class:open={openId === 'server'}><Icon name="CaretDown" size={12} weight="bold" /></span>
    </button>
    {#if openId === 'server'}
      <div class="body">
        {#if server.account}
          <div class="actions">
            <button class="btn press" onclick={() => nav.root({ name: 'server' })}>Open Server</button>
            <button class="btn press" onclick={serverSignOut}>Sign Out</button>
          </div>
        {:else}
          <p class="t-sub">Navidrome, Gonic, Airsonic, Ampache and others that speak the Subsonic API. Without https:// the address is used as http://, so type https:// if your server has TLS. The password is kept in Windows Credential Manager and only ever sent to that server, as a salted hash.</p>
          <form class="form" onsubmit={(e) => (e.preventDefault(), connectServer())}>
            <input bind:value={address} placeholder="Server address, e.g. 192.168.1.5:4533" aria-label="Server address" spellcheck="false" />
            <input bind:value={user} placeholder="User name" aria-label="User name" spellcheck="false" autocomplete="username" />
            <input bind:value={password} type="password" placeholder="Password" aria-label="Password" autocomplete="current-password" />
            <button class="btn primary press" disabled={!address.trim() || !user.trim() || !password || server.busy}>{server.busy ? 'Signing In…' : 'Sign In'}</button>
          </form>
        {/if}
        {#if server.error}<p class="err">{server.error}</p>{/if}
      </div>
    {/if}
  </section>

  <section class="prov">
    <button class="top press" aria-expanded={openId === 'spotify'} onclick={() => toggle('spotify')}>
      <span class="ic"><Icon name="SpotifyLogo" size={20} /></span>
      <span class="name">Spotify<span class="t-sub">Import playlists, control the Spotify app</span></span>
      <span class="state" class:on={!!spotify.user}>{spotify.user ? `Signed in as ${spotify.user}` : 'Sign in'}</span><span class="caret" class:open={openId === 'spotify'}><Icon name="CaretDown" size={12} weight="bold" /></span>
    </button>
    {#if openId === 'spotify'}
      <div class="body">
        {#if spotify.user}
          <div class="actions">
            <button class="btn press" onclick={() => nav.root({ name: 'spotify' })}>Open Spotify</button>
            <button class="btn press" onclick={() => disconnect()}>Sign Out</button>
          </div>
          <p class="t-sub">Signing out deletes Qistara's access. To remove it on Spotify's side too, use
            <button class="link" onclick={() => openUrl('https://www.spotify.com/account/apps/')}>spotify.com/account/apps</button>.</p>
        {:else if spotify.configured}
          <p class="t-sub">Approve Qistara in the browser window that opens. It never sees your Spotify password.</p>
          <div class="actions">
            <button class="btn primary press" onclick={connectSpotify} disabled={spotify.waiting}>{spotify.waiting ? 'Waiting for your browser…' : 'Sign In to Spotify…'}</button>
            <button class="btn press" onclick={() => disconnect(true)}>Use a Different Client ID</button>
          </div>
        {:else}
          <p class="t-sub">Spotify needs your own free developer app (your account must have Premium):</p>
          <ol class="steps t-sub">
            <li>Open the <button class="link" onclick={() => openUrl('https://developer.spotify.com/dashboard')}>Spotify Developer Dashboard</button> and choose <b>Create app</b>.</li>
            <li>Name it Qistara, add the Redirect URI <code>http://127.0.0.1/callback</code> and tick <b>Web API</b>.</li>
            <li>Copy the app's <b>Client ID</b> and paste it below. No client secret is needed.</li>
          </ol>
          <div class="keys">
            <input bind:value={clientId} placeholder="Client ID" aria-label="Spotify Client ID" spellcheck="false" />
            <button class="btn press" onclick={() => setClient(clientId.trim())} disabled={!clientId.trim()}>Save</button>
          </div>
        {/if}
        {#if spotify.error}<p class="err">{spotify.error}</p>{/if}
      </div>
    {/if}
  </section>
  <section class="prov">
    <button class="top press" aria-expanded={openId === 'apple'} onclick={() => toggle('apple')}>
      <span class="ic"><Icon name="AppleLogo" size={20} /></span>
      <span class="name">Apple Music<span class="t-sub">Import library playlists and songs</span></span>
      <span class="state" class:on={!!apple.s.user}>{apple.s.user ? 'Signed in' : 'Sign in'}</span><span class="caret" class:open={openId === 'apple'}><Icon name="CaretDown" size={12} weight="bold" /></span>
    </button>
    {#if openId === 'apple'}
      <div class="body">
        {#if apple.s.user}
          <div class="actions">
            <button class="btn press" onclick={() => nav.root({ name: 'apple' })}>Open Apple Music</button>
            <button class="btn press" onclick={() => apple.disconnect()}>Sign Out</button>
          </div>
        {:else if apple.s.configured}
          <p class="t-sub">A page opens in your browser: click Sign In there and approve with your Apple ID. Qistara never sees your password.</p>
          <div class="actions">
            <button class="btn primary press" onclick={apple.connect} disabled={apple.s.waiting}>{apple.s.waiting ? 'Waiting for your browser…' : 'Sign In to Apple Music…'}</button>
            <button class="btn press" onclick={() => apple.disconnect(true)}>Use a Different Key</button>
          </div>
        {:else}
          <p class="t-sub">Apple only lets apps read your library with a MusicKit key from the Apple Developer Program (a paid membership). Apple Music streams are DRM-protected, so the import matches songs to the files you have.</p>
          <ol class="steps t-sub">
            <li>In <button class="link" onclick={() => openUrl('https://developer.apple.com/account/resources/authkeys/list')}>Certificates, Identifiers &amp; Profiles › Keys</button>, create a key with <b>Media Services (MusicKit)</b> and download its <code>.p8</code> file.</li>
            <li>Enter your <b>Team ID</b> (top right of the developer site) and the key's <b>Key ID</b>, then choose the <code>.p8</code> file. It's kept in {KEY_STORE}.</li>
          </ol>
          <div class="keys">
            <input bind:value={teamId} placeholder="Team ID" aria-label="Apple Team ID" spellcheck="false" />
            <input bind:value={keyId} placeholder="Key ID" aria-label="MusicKit Key ID" spellcheck="false" />
            <button class="btn press" onclick={pickKey} disabled={!teamId.trim() || !keyId.trim()}>Choose .p8 Key…</button>
          </div>
        {/if}
        {#if apple.s.error}<p class="err">{apple.s.error}</p>{/if}
      </div>
    {/if}
  </section>

  <section class="prov">
    <button class="top press" aria-expanded={openId === 'tidal'} onclick={() => toggle('tidal')}>
      <span class="ic"><Icon name="Waves" size={20} /></span>
      <span class="name">Tidal<span class="t-sub">Import playlists and My Tracks</span></span>
      <span class="state" class:on={!!tidal.s.user}>{tidal.s.user ? `Signed in as ${tidal.s.user}` : 'Sign in'}</span><span class="caret" class:open={openId === 'tidal'}><Icon name="CaretDown" size={12} weight="bold" /></span>
    </button>
    {#if openId === 'tidal'}
      <div class="body">
        {#if tidal.s.user}
          <div class="actions">
            <button class="btn press" onclick={() => nav.root({ name: 'tidal' })}>Open Tidal</button>
            <button class="btn press" onclick={() => tidal.disconnect()}>Sign Out</button>
          </div>
        {:else if tidal.s.configured}
          <p class="t-sub">Approve Qistara in the browser window that opens. It never sees your Tidal password.</p>
          <div class="actions">
            <button class="btn primary press" onclick={tidal.connect} disabled={tidal.s.waiting}>{tidal.s.waiting ? 'Waiting for your browser…' : 'Sign In to Tidal…'}</button>
            <button class="btn press" onclick={() => tidal.disconnect(true)}>Use a Different Client ID</button>
          </div>
        {:else}
          <p class="t-sub">Tidal needs your own free developer app. Tidal's API lets other apps play only previews, so the import matches songs to the files you have.</p>
          <ol class="steps t-sub">
            <li>Open the <button class="link" onclick={() => openUrl('https://developer.tidal.com/dashboard')}>TIDAL Developer Dashboard</button> and create an app.</li>
            <li>Add the Redirect URI <code>{tidal.s.redirect || 'http://127.0.0.1:47823/callback'}</code> and the scopes <b>user.read</b>, <b>collection.read</b> and <b>playlists.read</b>.</li>
            <li>Paste the app's <b>Client ID</b> below. No client secret is needed.</li>
          </ol>
          <div class="keys">
            <input bind:value={tidalClient} placeholder="Client ID" aria-label="Tidal Client ID" spellcheck="false" />
            <button class="btn press" onclick={() => tidal.setup({ clientId: tidalClient.trim() })} disabled={!tidalClient.trim()}>Save</button>
          </div>
        {/if}
        {#if tidal.s.error}<p class="err">{tidal.s.error}</p>{/if}
      </div>
    {/if}
  </section>
</div>

<style>
  h1 { margin: 0 0 6px; }
  .lede { max-width: 640px; margin: 0 0 20px; line-height: 1.5; }
  .list { max-width: 760px; display: grid; gap: 6px; }
  .prov { border-radius: var(--r-lg); background: var(--fill); overflow: hidden; }
  .top { display: flex; align-items: center; gap: 14px; width: 100%; min-height: 58px; padding: 8px 14px; text-align: left; }
  .top:hover { background: var(--fill); }
  .ic { display: grid; place-items: center; width: 34px; height: 34px; border-radius: var(--r); background: var(--fill-2); color: var(--text); flex: none; }
  .name { flex: 1; min-width: 0; display: grid; font-weight: 600; }
  .name .t-sub { font-weight: 400; }
  .state { font-size: 12px; color: var(--text-2); white-space: nowrap; }
  .state.on { color: var(--accent); font-weight: 600; }
  .caret { display: grid; color: var(--text-3); transition: transform 160ms var(--ease-out); }
  .caret.open { transform: rotate(180deg); }
  .body { display: grid; gap: 12px; justify-items: start; padding: 4px 16px 16px 62px; }
  .body p { margin: 0; line-height: 1.5; max-width: 600px; }
  .choices { display: grid; gap: 6px; }
  .choice { display: flex; align-items: center; gap: 8px; min-width: 0; }
  .choice input { accent-color: var(--accent); }
  .path { max-width: 360px; }
  .actions { display: flex; gap: 8px; flex-wrap: wrap; }
  .btn:disabled { opacity: 0.6; }
  .link { color: var(--accent); text-decoration: underline; text-underline-offset: 2px; }
  .steps { margin: 0; padding-left: 20px; line-height: 1.7; }
  .keys { display: flex; gap: 8px; width: min(480px, 100%); }
  .keys input { flex: 1; min-width: 0; height: 30px; padding: 0 10px; border-radius: var(--r); border: 1px solid var(--sep); background: var(--bg-2); color: var(--text); }
  .form { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; width: min(560px, 100%); }
  .form input { height: 30px; padding: 0 10px; border-radius: var(--r); border: 1px solid var(--sep); background: var(--bg-2); color: var(--text); min-width: 0; }
  .form input:first-child { grid-column: 1 / -1; }
  .form .btn { justify-self: start; }
  code { font-size: 12px; padding: 1px 5px; border-radius: var(--r-xs); background: var(--fill-2); user-select: text; }
  .err { color: #e5484d; }
</style>
