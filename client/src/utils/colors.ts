export const colors = ['blue', 'green', 'orange', 'purple', 'red', 'white']
export default colors

export function randomColor() {
  return colors[Math.floor(Math.random() * colors.length)]
}

export function colorToHex(color) {
  return {
    blue: 0x00c2cc,
    green: 0x2ee53d,
    orange: 0xf2d91a,
    purple: 0x8a2ee5,
    red: 0xe5482e,
    white: 0xe5feff,
  }[color]
}

export function colorToHexString(color) {
  return {
    blue: '#00c2cc',
    green: '#2ee53d',
    orange: '#f2d91a',
    purple: '#8a2ee5',
    red: '#e5482e',
    white: '#e5feff',
  }[color]
}
