import type { Color, Emote, Mutator, Round, Shape } from '@/types'

// From src/server/arena/round.rs.
export const LAYOUT_CELLS = 14
export const SHRINK_DELAY_SECONDS = 10
export const SHRINK_RATE = 4
export const SHRINK_MIN_SIZE = 160

export const mutatorInfo: Record<Mutator, { name: string; description: string; color: Color }> = {
  Shrink: { name: 'Shrink', description: `Walls close in after ${SHRINK_DELAY_SECONDS}s`, color: 'red' },
  Wrap: { name: 'Wraparound', description: 'Ride off an edge, come back on the other side', color: 'purple' },
  Snake: { name: 'Snake', description: 'Trails stay short', color: 'green' },
  Overdrive: { name: 'Overdrive', description: 'Everyone rides faster', color: 'orange' },
  Fog: { name: 'Fog', description: 'You only see what is near you', color: 'white' },
  Obstacles: { name: 'Obstacles', description: 'Blocks scattered about the grid', color: 'blue' },
}

export const shapeNames: Record<Shape, string> = {
  Grid: 'Grid',
  Circle: 'Circle',
  Pillars: 'Pillars',
  Cross: 'Cross',
  Core: 'Core',
  Bastions: 'Bastions',
}

export const emoteLabels: Record<Emote, string> = { gg: 'GG', lol: 'LOL', rip: 'RIP', wtf: '!?!' }
export const emotes: Emote[] = ['gg', 'lol', 'rip', 'wtf']

export function hasMutator(round: Round, mutator: Mutator) {
  return round.mutators.includes(mutator)
}

// How far the walls have closed in on each side, in arena units, `elapsed`
// seconds into a round of an arena `size` across, as the server works it out.
export function shrinkMargin(size: number, elapsed: number) {
  const maxMargin = Math.max(0, (size - SHRINK_MIN_SIZE) / 2)
  return Math.min(maxMargin, Math.max(0, (elapsed - SHRINK_DELAY_SECONDS) * SHRINK_RATE))
}

export function cellKey([x, y]: [number, number]) {
  return `${x},${y}`
}

// The sides of walled-off cells that face open cells inside the arena, in
// cell units, as [x1, y1, x2, y2].
export function wallOutline(blocked: [number, number][]): [number, number, number, number][] {
  const cells = new Set(blocked.map(cellKey))
  const open = (x: number, y: number) =>
    x >= 0 && y >= 0 && x < LAYOUT_CELLS && y < LAYOUT_CELLS && !cells.has(cellKey([x, y]))
  const sides: [number, number, number, number][] = []
  for (const [x, y] of blocked) {
    if (open(x - 1, y)) sides.push([x, y, x, y + 1])
    if (open(x + 1, y)) sides.push([x + 1, y, x + 1, y + 1])
    if (open(x, y - 1)) sides.push([x, y, x + 1, y])
    if (open(x, y + 1)) sides.push([x, y + 1, x + 1, y + 1])
  }
  return sides
}
