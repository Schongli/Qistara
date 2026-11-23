/** A YouTube video id from a link or a bare id: watch?v=, youtu.be/, /shorts/, /embed/, /live/. */
export function parseVideoId(s: string): string | null {
  const t = s.trim()
  if (/^[\w-]{11}$/.test(t)) return t
  const m = t.match(/(?:[?&]v=|youtu\.be\/|\/shorts\/|\/embed\/|\/live\/)([\w-]{11})(?![\w-])/)
  return m?.[1] ?? null
}
