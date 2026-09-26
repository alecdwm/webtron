import type { Color } from '@/types'

export const colors: Color[] = ['blue', 'green', 'orange', 'purple', 'red', 'white']
export default colors

export function randomColor(): Color {
  return colors[Math.floor(Math.random() * colors.length)]
}

const hexStrings: Record<Color, string> = {
  blue: '#00c2cc',
  green: '#2ee53d',
  orange: '#f2d91a',
  purple: '#8a2ee5',
  red: '#e5482e',
  white: '#e5feff',
}

export function colorToHexString(color: Color): string {
  return hexStrings[color]
}
