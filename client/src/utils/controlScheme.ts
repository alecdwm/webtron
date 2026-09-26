import type { ControlScheme } from '@/types'

const CONTROL_SCHEME_STORAGE_KEY = 'webtron:controls'

// The player's choice if they have made one. Otherwise steering on devices
// whose main input is a touchscreen, where tapping either side of the arena is
// easier than tapping towards a direction, and directional everywhere else.
export function readControlScheme(): ControlScheme {
  let stored: string | null = null
  try {
    stored = window.localStorage.getItem(CONTROL_SCHEME_STORAGE_KEY)
  } catch {
    // Storage may be unavailable (private mode); fall back to the default.
  }
  if (stored === 'directional' || stored === 'steering') return stored
  return window.matchMedia('(pointer: coarse)').matches ? 'steering' : 'directional'
}

export function storeControlScheme(controlScheme: ControlScheme) {
  try {
    window.localStorage.setItem(CONTROL_SCHEME_STORAGE_KEY, controlScheme)
  } catch {
    // Storage may be unavailable (private mode); the setting just won't persist.
  }
}
