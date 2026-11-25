import { describe, expect, it } from 'vitest'
import { playbackHelp } from './playbackhelp'

// what the plugin and libmpv-wrapper report when a library can't be loaded
const NO_LIBMPV_MAC = 'FFI error: libmpv library load failed: DlOpen { desc: "dlopen(/Applications/Qistara.app/Contents/MacOS/lib/libmpv.dylib, 0x0005): Library not loaded: /opt/homebrew/opt/mpv/lib/libmpv.2.dylib" }'
const NO_LIBMPV_LINUX = 'FFI error: libmpv library load failed: DlOpen { desc: "libmpv.so.2: cannot open shared object file: No such file or directory" }'
const NO_WRAPPER = "FFI error: Failed to load libmpv-wrapper from 'libmpv-wrapper.so'. Error: DlOpen { desc: \"libmpv-wrapper.so: cannot open shared object file\" }"

const commands = (h: ReturnType<typeof playbackHelp>) => h.steps.flatMap((s) => s.commands?.map((c) => c.command) ?? [])

describe('playbackHelp', () => {
  it('asks a Mac without Homebrew mpv to install it, then restart', () => {
    const h = playbackHelp('macos', NO_LIBMPV_MAC)
    expect(h.missing).toBe(true)
    expect(commands(h)).toEqual(['brew install mpv'])
    expect(h.steps[0].link?.url).toBe('https://brew.sh')
    expect(h.detail).toBe(NO_LIBMPV_MAC)
  })

  it('gives Linux the package for each family of distributions', () => {
    const h = playbackHelp('linux', NO_LIBMPV_LINUX)
    expect(h.missing).toBe(true)
    expect(commands(h)).toEqual(['sudo apt install libmpv2', 'sudo dnf install mpv-libs', 'sudo pacman -S mpv', 'sudo zypper install libmpv2'])
  })

  it('says to reinstall when Qistara’s own part is missing, not to install mpv', () => {
    for (const os of ['windows', 'macos', 'linux'] as const) {
      const h = playbackHelp(os, NO_WRAPPER)
      expect(h.missing).toBe(false)
      expect(commands(h)).toEqual([])
      expect(h.steps.some((s) => s.link?.url.includes('/releases'))).toBe(true)
    }
  })

  it('never tells Windows to install mpv: it comes with the installer', () => {
    const h = playbackHelp('windows', 'FFI error: libmpv library load failed: DlOpen { desc: "libmpv-2.dll" }')
    expect(h.missing).toBe(false)
    expect(commands(h)).toEqual([])
  })

  it('shows any other error as it is, with a restart', () => {
    const h = playbackHelp('linux', 'Error parsing option vo')
    expect(h.missing).toBe(false)
    expect(h.title).toBe("The player didn't start")
    expect(h.detail).toBe('Error parsing option vo')
  })
})
