import type { Color } from '@/types'

export const colors: Color[] = ['blue', 'green', 'orange', 'purple', 'red', 'white']
export default colors

export function randomColor(): Color {
  return colors[Math.floor(Math.random() * colors.length)]
}

// The color's custom property from index.css, which holds a wide-gamut version
// of it on displays that can show one.
export function colorToCss(color: Color): string {
  return `var(--${color})`
}
