// What to do when the player didn't start (mpv.error, PlaybackSetup.svelte). Windows ships mpv in the installer;
// macOS uses Homebrew's mpv and Linux the system's libmpv (scripts/setup-libmpv.sh), which may not be installed yet.
import type { Os } from './signal'

export interface Step {
  text: string
  /** commands to copy into a terminal, each with the systems it's for */
  commands?: { label?: string; command: string }[]
  link?: { label: string; url: string }
}
export interface PlaybackHelp {
  title: string
  /** mpv itself isn't installed (macOS, Linux): installing it and restarting fixes it */
  missing: boolean
  steps: Step[]
  /** the error as reported, for a bug report */
  detail: string
}

const RELEASES = { label: 'Qistara’s releases', url: 'https://github.com/Schongli/Qistara/releases' }
const RESTART: Step = { text: 'Then restart Qistara.' }

export function playbackHelp(os: Os, error: string): PlaybackHelp {
  // the plugin's own part (libmpv-wrapper) comes with Qistara; libmpv is the one the system provides
  const wrapper = /libmpv-wrapper/i.test(error)
  const libmpv = !wrapper && /libmpv library load failed|libmpv\.so|libmpv(\.2)?\.dylib/i.test(error)
  if (libmpv && os === 'macos') {
    return {
      title: 'Install mpv to play music',
      missing: true,
      steps: [
        { text: 'Qistara plays through mpv, from Homebrew. If you don’t have Homebrew yet, install it first:', link: { label: 'brew.sh', url: 'https://brew.sh' } },
        { text: 'Then, in Terminal:', commands: [{ command: 'brew install mpv' }] },
        RESTART,
      ],
      detail: error,
    }
  }
  if (libmpv && os === 'linux') {
    return {
      title: 'Install libmpv to play music',
      missing: true,
      steps: [
        {
          text: 'Qistara plays through your system’s libmpv. In a terminal, run the line for your system:',
          commands: [
            { label: 'Ubuntu, Debian, Mint', command: 'sudo apt install libmpv2' },
            { label: 'Fedora', command: 'sudo dnf install mpv-libs' },
            { label: 'Arch, Manjaro', command: 'sudo pacman -S mpv' },
            { label: 'openSUSE', command: 'sudo zypper install libmpv2' },
          ],
        },
        RESTART,
      ],
      detail: error,
    }
  }
  if (wrapper || libmpv) {
    return {
      title: 'Part of Qistara is missing',
      missing: false,
      steps: [{ text: 'The player that came with Qistara couldn’t be found. Download Qistara again and reinstall it:', link: RELEASES }],
      detail: error,
    }
  }
  return {
    title: "The player didn't start",
    missing: false,
    steps: [{ text: 'Restart Qistara. If this keeps happening, the log has the details for a bug report.' }],
    detail: error,
  }
}
