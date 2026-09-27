import type { Color, Direction, Emote, Mutator, Point, Portal, Round, Shape } from '@/types'

// From src/server/arena/round.rs.
export const LAYOUT_CELLS = 14
export const SHRINK_DELAY_SECONDS = 10
export const SHRINK_RATE = 4
export const SHRINK_MIN_SIZE = 160
// How long the countdown before a round is, from ARENA_START_TIMER_SECONDS in
// src/server/arena.rs.
export const COUNTDOWN_MS = 3000

// Portals appear one after another during the countdown, once the
// lightcycles have rezzed in (see spawnDelayMs in utils/spawn.ts), spaced out
// so that each one's sound can be heard: the ends of each grow in over
// PORTAL_ENDS_MS, then its surface ripples in from them over
// PORTAL_SURFACE_MS.
export const PORTAL_ENDS_MS = 400
export const PORTAL_SURFACE_MS = 700
const FIRST_PORTAL_MS = 650
const PORTAL_STAGGER_MS = 150

// When portal number `index` of a round starting at `startAt`, counting both
// portals of each pair in turn, begins to appear, in server time.
export function portalAppearAt(startAt: number, index: number) {
  return startAt - COUNTDOWN_MS + FIRST_PORTAL_MS + index * PORTAL_STAGGER_MS
}

export const mutatorInfo: Record<Mutator, { name: string; description: string; color: Color }> = {
  Shrink: { name: 'Shrink', description: `Walls close in after ${SHRINK_DELAY_SECONDS}s`, color: 'red' },
  Wrap: { name: 'Wraparound', description: 'Ride off an edge, come back on the other side', color: 'purple' },
  Snake: { name: 'Snake', description: 'Trails stay short', color: 'green' },
  Overdrive: { name: 'Overdrive', description: 'Everyone rides faster', color: 'orange' },
  Fog: { name: 'Fog', description: 'You only see what is near you', color: 'white' },
  Obstacles: { name: 'Obstacles', description: 'Blocks scattered about the grid', color: 'blue' },
  Portals: { name: 'Portals', description: 'Ride into a portal, come out of its twin', color: 'purple' },
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
// The emotes players can send. NPCs also use 'wtf' when they start a grudge.
export const emotes: Emote[] = ['lol', 'rip', 'gg']

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

export const velocities: Record<Direction, Point> = {
  up: [0, 1],
  down: [0, -1],
  left: [-1, 0],
  right: [1, 0],
}

// The corners of a rectangle, the lowest first.
export type Rect = [Point, Point]

// What sends a lightcycle somewhere else: the size of an arena that wraps
// round, or null, and the ends of each pair of portals, the lowest first.
export type Gates = { wrap: Point | null; portals: [Rect, Rect][] }

export const noGates: Gates = { wrap: null, portals: [] }

export function gatesOf(round: Round, width: number, height: number): Gates {
  const cellWidth = width / LAYOUT_CELLS
  const cellHeight = height / LAYOUT_CELLS
  const rect = ({ x, y, length, upright }: Portal): Rect => {
    const start: Point = [x * cellWidth, y * cellHeight]
    return [start, upright ? [start[0], start[1] + length * cellHeight] : [start[0] + length * cellWidth, start[1]]]
  }
  return {
    wrap: hasMutator(round, 'Wrap') ? [width, height] : null,
    portals: round.portals.map(({ twins: [a, b] }) => [rect(a), rect(b)]),
  }
}

// The most stretches a straight line is followed through, as on the server.
const MAX_STRETCHES = 32

// Where a lightcycle at `position` going `direction` ends up after
// `distance`, through any portals or wrapping edges on the way, and whether
// it went through one, as the server works it out (see Arena::stretches in
// src/server/arena.rs).
export function advance(
  position: Point,
  direction: Direction,
  distance: number,
  gates: Gates,
): { point: Point; jumped: boolean } {
  const [dx, dy] = velocities[direction]
  let from = position
  let left = distance
  let jumped = false
  for (let stretch = 1; stretch < MAX_STRETCHES; stretch++) {
    const gate = nextGate(from, direction, gates, left)
    if (gate === null) break
    left -= gate.distance
    from = gate.exit
    jumped = true
  }
  left = Math.max(0, left)
  return { point: [from[0] + dx * left, from[1] + dy * left], jumped }
}

// How far from `from` in `direction` the nearest portal or wrapping edge
// within `reach` is, and where a lightcycle going into it comes out. A portal
// is only gone through across it, not along it, once reached, even at the end
// of the reach, but not from on it, as on the server (see Arena::next_gate in
// src/server/arena.rs).
function nextGate(from: Point, direction: Direction, { wrap, portals }: Gates, reach: number) {
  let nearest: { distance: number; exit: Point } | null = null
  const consider = (distance: number, exit: Point, portal = false) => {
    const within = portal ? distance > 0 && distance <= reach : distance < reach
    if (within && (nearest === null || distance < nearest.distance)) nearest = { distance, exit }
  }
  const [x, y] = from

  if (wrap) {
    const [width, height] = wrap
    if (direction === 'up') consider(Math.max(0, height - y), [x, 0])
    if (direction === 'down') consider(Math.max(0, y), [x, height])
    if (direction === 'right') consider(Math.max(0, width - x), [0, y])
    if (direction === 'left') consider(Math.max(0, x), [width, y])
  }

  for (const [a, b] of portals) {
    for (const [[min, max], [outMin, outMax]] of [
      [a, b],
      [b, a],
    ]) {
      const across = x - min[0] + outMin[0]
      const up = y - min[1] + outMin[1]
      const besideX = min[0] < x && x < max[0]
      const besideY = min[1] < y && y < max[1]
      if (direction === 'right' && besideY && x <= min[0]) consider(min[0] - x, [outMax[0], up], true)
      if (direction === 'left' && besideY && x >= max[0]) consider(x - max[0], [outMin[0], up], true)
      if (direction === 'up' && besideX && y <= min[1]) consider(min[1] - y, [across, outMax[1]], true)
      if (direction === 'down' && besideX && y >= max[1]) consider(y - max[1], [across, outMin[1]], true)
    }
  }

  return nearest as { distance: number; exit: Point } | null
}
