import type { Color } from '@/types'
import colors, { randomColor } from '@/utils/colors'

const PLAYER_NAME_STORAGE_KEY = 'webtron:name'
const PLAYER_COLOR_STORAGE_KEY = 'webtron:color'

function read(key: string): string | null {
  try {
    return window.localStorage.getItem(key)
  } catch {
    // Storage may be unavailable (private mode); fall back to the default.
    return null
  }
}

function store(key: string, value: string) {
  try {
    window.localStorage.setItem(key, value)
  } catch {
    // Storage may be unavailable (private mode); the setting just won't persist.
  }
}

// The name the player last entered, or blank.
export function readPlayerName(): string {
  return read(PLAYER_NAME_STORAGE_KEY) ?? ''
}

export function storePlayerName(name: string) {
  store(PLAYER_NAME_STORAGE_KEY, name)
}

// The color the player last had. On their first visit this is a random color,
// which is stored so they keep it on later visits until they pick another.
export function readPlayerColor(): Color {
  const stored = read(PLAYER_COLOR_STORAGE_KEY)
  const color = colors.find((color) => color === stored)
  if (color) return color
  const random = randomColor()
  storePlayerColor(random)
  return random
}

export function storePlayerColor(color: Color) {
  store(PLAYER_COLOR_STORAGE_KEY, color)
}
