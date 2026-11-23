// Listening together, the guest's side as pure logic: from what the host says is playing, what this player should
// do. Kept apart from session.svelte.ts so every case has a test.

export interface HostItem {
  seq: number
  kind: 'file' | 'yt' | 'url'
  title: string
  artist: string
  album: string
  duration: number
  id?: string // yt: the video
  video?: boolean // yt: played with its picture
  mv?: { yt: string; offset: number } // file: the YouTube music video shown over it, in step
  url?: string // url: the stream
  art?: string | null // https (yt) or a small data:image/jpeg (file)
  codec?: string
  sampleRate?: number | null
  bitDepth?: number | null
  lossless?: boolean
}
export interface HostState { cur: HostItem | null; next: HostItem | null; pos: number; paused: boolean }
/** What this player is doing. `nextSeq`: the item lined up after the current one (gapless). */
export interface Mine { curSeq: number | null; nextSeq: number | null; time: number; duration: number; paused: boolean }
export type Action =
  | { do: 'stop' }
  | { do: 'load'; at: number }
  | { do: 'seek'; at: number }
  | { do: 'pause'; paused: boolean }
  | { do: 'next' }

/** Off by more than this, a guest seeks to catch up (less is inaudible, and seeking itself costs a moment). */
export const DRIFT = 1
/** This close to its end, a guest's own gapless change to the next song is left to happen. */
export const HANDOVER = 2

/** Where the host is now: its position plus, while playing, the time since it said so (`since`, seconds). */
export const hostTime = (h: HostState, since: number) => h.pos + (h.paused ? 0 : since)

export function plan(me: Mine, host: HostState, since: number): Action[] {
  if (!host.cur) return me.curSeq === null ? [] : [{ do: 'stop' }]
  const at = Math.max(0, hostTime(host, since))
  const wantNext = host.next?.seq ?? null
  if (me.curSeq !== host.cur.seq) {
    // the host moved on to the song this player has lined up and is about to reach: let it play into it
    if (me.nextSeq === host.cur.seq && !host.paused && me.duration > 0 && me.duration - me.time <= HANDOVER) return []
    return [{ do: 'load', at }, ...(host.paused ? [{ do: 'pause', paused: true } as Action] : [])]
  }
  const out: Action[] = []
  if (Math.abs(me.time - at) > DRIFT) out.push({ do: 'seek', at })
  if (me.paused !== host.paused) out.push({ do: 'pause', paused: host.paused })
  if (me.nextSeq !== wantNext) out.push({ do: 'next' })
  return out
}
