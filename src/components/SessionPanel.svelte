<script lang="ts">
  // Listening together, in the side pane (white on the blurred cover like lyrics and Up Next): start or join a
  // session, the invite, who's here, and the chat.
  import Icon from './Icon.svelte'
  import { chat, host, join, kick, leave, session } from '../lib/session.svelte'
  import { queue } from '../lib/queue.svelte'
  import { tick } from 'svelte'

  let name = $state('')
  let port = $state('')
  let code = $state('')
  let text = $state('')
  let list = $state<HTMLDivElement>()
  let copied = $state(-1)

  async function copy(i: number) {
    await navigator.clipboard.writeText(session.invites[i]).catch(() => {})
    copied = i
    setTimeout(() => (copied = -1), 1600)
  }
  async function send() {
    if (!text.trim()) return
    await chat(text)
    text = ''
  }
  $effect(() => {
    session.chat.length
    tick().then(() => list?.scrollTo({ top: list.scrollHeight }))
  })
  const time = (ms: number) => new Date(ms).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
</script>

<div class="sess">
  {#if !session.role}
    <h3>Listen together</h3>
    <p class="sub">Everyone hears the same song at the same moment, with a chat beside it. Your PC is the host: guests on your network, your VPN (Tailscale, ZeroTier) or through a forwarded port join with the invite. Your own files play for them too.</p>
    <label class="field"><span>Your name</span><input bind:value={name} maxlength="32" placeholder="Shown to everyone" /></label>

    <div class="card">
      <b>Host</b>
      <label class="field"><span>Port</span><input bind:value={port} inputmode="numeric" placeholder="47800" /></label>
      <button class="go press" disabled={session.busy} onclick={() => host(name.trim() || 'Host', Number(port) || null)}><Icon name="Users" size={15} />Start a Session</button>
      <p class="sub small">Your system may ask to let Qistara accept connections: allow it for private networks.</p>
    </div>

    <div class="card">
      <b>Join</b>
      <label class="field"><span>Invite</span><input bind:value={code} placeholder="qistara://session/…" spellcheck="false" /></label>
      <button class="go press" disabled={session.busy || !code.trim()} onclick={() => join(code, name.trim())}><Icon name="Play" size={14} weight="fill" />Join</button>
    </div>
    {#if session.error}<p class="err">{session.error}</p>{/if}
  {:else}
    <div class="top">
      <div>
        <h3>{session.role === 'host' ? 'Your session' : `${session.host}'s session`}</h3>
        <span class="sub">{session.guests.length} {session.guests.length === 1 ? 'guest' : 'guests'}{session.role === 'guest' ? ' · you follow the host' : ''}</span>
      </div>
      <button class="leave press" onclick={leave}>{session.role === 'host' ? 'End' : 'Leave'}</button>
    </div>

    {#if session.role === 'host'}
      <div class="invites">
        {#each session.invites as inv, i (inv)}
          <button class="invite press" onclick={() => copy(i)} title="Copy">
            <span class="code">{inv.replace(/#.*/, '#…')}</span><span class="copy">{copied === i ? 'Copied' : 'Copy'}</span>
          </button>
        {/each}
        <p class="sub small">The part after # is the key: share the invite only with people you'd let in.</p>
      </div>
    {/if}

    <div class="people">
      {#if session.role === 'guest'}<span class="who host">{session.host}</span>{/if}
      {#each session.guests as g (g.id)}
        <span class="who">{g.name}{#if session.role === 'host'}<button class="x" aria-label="Remove {g.name}" title="Remove {g.name}" onclick={() => kick(g.id)}><Icon name="X" size={10} weight="bold" /></button>{/if}</span>
      {/each}
    </div>
    {#if session.role === 'guest' && queue.current}
      <p class="sub now">Now: <b>{queue.current.title}</b>{queue.current.artist ? ` · ${queue.current.artist}` : ''}</p>
    {/if}

    <div class="chat" bind:this={list} aria-live="polite">
      {#each session.chat as m, i (i)}
        <p class="msg" class:me={m.me}><b>{m.from}</b> <span>{m.text}</span> <time>{time(m.at)}</time></p>
      {:else}
        <p class="sub">Say hello.</p>
      {/each}
    </div>
    <form class="say" onsubmit={(e) => (e.preventDefault(), send())}>
      <input bind:value={text} maxlength="500" placeholder="Message" aria-label="Message" />
      <button class="send press" aria-label="Send" disabled={!text.trim()}><Icon name="ChatCircle" size={16} weight="fill" /></button>
    </form>
  {/if}
</div>

<style>
  .sess { display: flex; flex-direction: column; gap: 12px; height: 100%; padding: 12px 14px 14px; color: #fff; overflow-y: auto; }
  h3 { margin: 0; font-size: 16px; }
  .sub { margin: 0; font-size: 12.5px; line-height: 1.45; color: rgba(255, 255, 255, 0.66); }
  .small { font-size: 11.5px; }
  .field { display: grid; gap: 4px; font-size: 12px; color: rgba(255, 255, 255, 0.66); }
  input { height: 34px; padding: 0 10px; border-radius: var(--r); border: 1px solid rgba(255, 255, 255, 0.18); background: rgba(0, 0, 0, 0.25); color: #fff; font: inherit; font-size: 13px; }
  input:focus { outline: none; border-color: rgba(255, 255, 255, 0.6); }
  .card { display: grid; gap: 8px; padding: 12px; border-radius: var(--r-lg); background: rgba(255, 255, 255, 0.08); }
  .go { display: flex; align-items: center; justify-content: center; gap: 6px; height: 34px; border-radius: var(--r); background: #fff; color: #111; font-weight: 650; }
  .go:disabled { opacity: 0.5; }
  .err { margin: 0; color: #ffb4b4; font-size: 12.5px; }
  .top { display: flex; align-items: flex-start; justify-content: space-between; gap: 8px; }
  .leave { height: 28px; padding: 0 12px; border-radius: 14px; background: rgba(255, 255, 255, 0.14); font-weight: 600; font-size: 12.5px; }
  .invites { display: grid; gap: 6px; }
  .invite { display: flex; align-items: center; gap: 8px; padding: 8px 10px; border-radius: var(--r); background: rgba(255, 255, 255, 0.1); text-align: left; min-width: 0; }
  .code { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font: 11.5px ui-monospace, 'Cascadia Mono', Consolas, monospace; }
  .copy { font-size: 11.5px; font-weight: 650; color: rgba(255, 255, 255, 0.8); }
  .people { display: flex; flex-wrap: wrap; gap: 6px; }
  .who { display: inline-flex; align-items: center; gap: 4px; height: 24px; padding: 0 10px; border-radius: 12px; background: rgba(255, 255, 255, 0.12); font-size: 12px; font-weight: 600; }
  .who.host { background: rgba(255, 255, 255, 0.24); }
  .x { display: grid; place-items: center; width: 16px; height: 16px; margin-right: -4px; border-radius: 50%; }
  .x:hover { background: rgba(255, 255, 255, 0.2); }
  .now { margin: 0; }
  .chat { flex: 1; min-height: 120px; overflow-y: auto; display: grid; align-content: start; gap: 6px; padding: 8px; border-radius: var(--r-lg); background: rgba(0, 0, 0, 0.22); }
  .msg { margin: 0; font-size: 13px; line-height: 1.4; overflow-wrap: anywhere; user-select: text; }
  .msg b { font-weight: 650; }
  .msg.me b { color: var(--accent); }
  .msg time { font-size: 10.5px; color: rgba(255, 255, 255, 0.45); }
  .say { display: flex; gap: 6px; }
  .say input { flex: 1; min-width: 0; }
  .send { display: grid; place-items: center; width: 34px; height: 34px; border-radius: var(--r); background: rgba(255, 255, 255, 0.16); }
  .send:disabled { opacity: 0.4; }
</style>
