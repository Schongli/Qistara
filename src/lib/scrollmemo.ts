// Scroll positions for Back/Forward. The page scroller keeps an anchor (the first visible [data-key] item and
// its offset) as well as pixels, because album tiles use content-visibility with an estimated height, so a
// remounted grid can be a little taller or shorter above the fold. Inner scrollers ([data-scroll]: virtual
// lists, split panes, Home's shelves) are kept by their order on the page, which a view repeats when remounted.

export interface Snap {
  top: number
  anchor?: { key: string; dy: number }
  inner: [number, number][]
}
/** What the page that is now showing should do: go back to a snapshot, start at the top, or stay put. */
export type Restore = Snap | 'top' | 'keep'

export function capture(root: HTMLElement): Snap {
  const inner = [...root.querySelectorAll<HTMLElement>('[data-scroll]')].map((el): [number, number] => [el.scrollTop, el.scrollLeft])
  if (root.scrollTop <= 0) return { top: 0, inner }
  const edge = root.getBoundingClientRect().top
  for (const el of root.querySelectorAll<HTMLElement>('[data-key]')) {
    const b = el.getBoundingClientRect()
    if (b.bottom > edge) return { top: root.scrollTop, anchor: { key: el.dataset.key!, dy: b.top - edge }, inner }
  }
  return { top: root.scrollTop, inner }
}

export function restore(root: HTMLElement, to: Restore) {
  if (to === 'keep') return
  if (to === 'top') return root.scrollTo({ top: 0 })
  root.scrollTop = to.top
  const el = to.anchor && root.querySelector<HTMLElement>(`[data-key="${CSS.escape(to.anchor.key)}"]`)
  if (el) root.scrollTop += el.getBoundingClientRect().top - root.getBoundingClientRect().top - to.anchor!.dy
  root.querySelectorAll<HTMLElement>('[data-scroll]').forEach((el, i) => {
    if (to.inner[i]) [el.scrollTop, el.scrollLeft] = to.inner[i]
  })
}
