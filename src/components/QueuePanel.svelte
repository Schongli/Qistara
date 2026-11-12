<script lang="ts">
  import QueueList from './QueueList.svelte'
  import { ui } from '../lib/nav.svelte'
</script>

<!-- a parallel panel: no scrim, slides in and out on the same right-hand path -->
<aside class="panel" class:open={ui.queue} aria-label="Up Next" aria-hidden={!ui.queue} inert={!ui.queue}>
  <QueueList />
</aside>

<style>
  .panel {
    position: absolute; z-index: 9; top: var(--bar-h); right: 0; bottom: 0; width: 320px; overflow-y: auto;
    padding: 4px 10px 20px; background: var(--bar); backdrop-filter: blur(30px) saturate(180%);
    border-left: 1px solid var(--sep); box-shadow: var(--shadow-pop);
    transform: translateX(105%); transition: transform 320ms var(--ease-out);
  }
  .panel.open { transform: none; }
  @media (prefers-reduced-motion: reduce) {
    .panel { transform: none; opacity: 0; transition: opacity 150ms; }
    .panel.open { opacity: 1; }
  }
  @media (prefers-reduced-transparency: reduce) { .panel { background: var(--bg-2); backdrop-filter: none; } }
</style>
