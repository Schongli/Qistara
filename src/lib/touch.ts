// Touch screens: one tap plays a row a mouse double-clicks, and a long press opens the menu a right-click does
// (Android's WebView does that by itself; iOS's doesn't, so it's done here, once for every element).

const coarse = typeof matchMedia === 'undefined' ? null : matchMedia('(pointer: coarse)')

/** Play on double-click, or on a single tap on a touch screen: spread onto the row, `{...activate(play)}`. */
export const activate = (fn: () => void) => ({
  ondblclick: fn,
  onclick: (e: MouseEvent) => {
    if (coarse?.matches || (e as PointerEvent).pointerType === 'touch') fn()
  },
})

if (typeof document !== 'undefined') {
  let timer = 0
  let start: PointerEvent | null = null
  let fired = false
  const cancel = () => (clearTimeout(timer), (start = null))
  document.addEventListener('pointerdown', (e) => {
    if (e.pointerType !== 'touch') return
    cancel()
    start = e
    fired = false
    timer = window.setTimeout(() => {
      if (!start) return
      fired = true
      start.target?.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: start.clientX, clientY: start.clientY }))
      start = null
    }, 500)
  }, { passive: true })
  document.addEventListener('pointermove', (e) => {
    if (start && Math.hypot(e.clientX - start.clientX, e.clientY - start.clientY) > 10) cancel()
  }, { passive: true })
  document.addEventListener('pointerup', cancel)
  document.addEventListener('pointercancel', cancel)
  // the WebView opened the menu by itself: don't open it twice
  document.addEventListener('contextmenu', (e) => e.isTrusted && cancel(), true)
  // the tap that ends a long press isn't also a tap on the row
  document.addEventListener('click', (e) => {
    if (fired) (e.stopPropagation(), e.preventDefault(), (fired = false))
  }, true)
}
