import { describe, expect, it } from 'vitest'
import { nav } from './nav.svelte'
import type { Snap } from './scrollmemo'

describe('navigation history', () => {
  it('returns to where each page was scrolled, and opens new pages at the top', () => {
    let y = 0
    nav.capture = (): Snap => ({ top: y, inner: [] })
    nav.root({ name: 'albums' })
    expect(nav.restore).toBe('top')
    y = 900
    nav.go({ name: 'album', key: 'a' })
    expect(nav.restore).toBe('top')
    y = 40
    nav.back()
    expect(nav.route).toEqual({ name: 'albums' })
    expect(nav.restore).toEqual({ top: 900, inner: [] })
    nav.forward()
    expect(nav.route).toEqual({ name: 'album', key: 'a' })
    expect(nav.restore).toEqual({ top: 40, inner: [] })
  })

  it('changes the selection without history or scrolling', () => {
    nav.root({ name: 'artists' })
    nav.replace({ name: 'artists', artist: 'B' })
    expect(nav.restore).toBe('keep')
    expect(nav.canBack).toBe(false)
    nav.go({ name: 'album', key: 'x' })
    nav.back()
    expect(nav.route).toEqual({ name: 'artists', artist: 'B' })
  })
})
