// A streamed song that stops before its end (its stream dropped and couldn't be picked up again) mustn't count as
// finished: youtube.svelte.ts opens it again where it stopped instead of moving on to the next song.

/** mpv ended a file early: an error, or the end of the stream well before the song's end. */
export function endedEarly(reason: string, time: number, duration: number): boolean {
  if (reason === 'error') return true
  return reason === 'eof' && duration > 0 && time < duration - 5
}

/** The video id in the streaming engine's address for a song (ytstream.rs: http://127.0.0.1:<port>/<token>/yt/<id>). */
export const streamedId = (path: string) => path.match(/^http:\/\/127\.0\.0\.1:\d+\/[0-9a-f]+\/yt\/([\w-]{11})$/)?.[1] ?? null
