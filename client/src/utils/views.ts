import type { Direction, Lightribbon, Point } from '@/types'
import { type Gates, velocities } from '@/utils/round'

// How a lightcycle and its headlight are seen through portals and the edges
// of an arena that wraps round, as copies of it drawn elsewhere.
//
// Going into a gate (either side of a portal, or an edge of an arena that
// wraps round) comes out of the gate it is joined to, which is somewhere else
// by a fixed shift. So a copy of the lightcycle drawn that shift away, and
// shown only in the part of the arena reached through the gate, looks like
// what went in coming out, and the lightcycle itself is hidden in the part
// of the arena behind the gate. The sprite goes through where it has crossed
// the gate's line, level with the gate. The headlight's light goes through
// where it is seen from the lamp through the gate, so a gate always casts a
// shadow in the light, which comes out of the other gate instead.
//
// A copy can go through gates in turn, which makes copies of copies.
// Measured from a copy, the straight-line distance to anything it lights is
// how far the light travelled to get there, so following gates only while
// they are within VIEW_REACH of each copy stops once the light has run out,
// however many gates it goes through.

export type Polygon = Point[]

// How far, in arena units, light is followed from a lightcycle: as far as its
// beam reaches at full boost.
const VIEW_REACH = 260
// The most copies of one lightcycle drawn, however many gates there are.
export const MAX_COPIES = 12
// How wide the headlight's beam spreads either side of straight ahead, a
// little wider than it is drawn.
const BEAM_SPREAD = (40 * Math.PI) / 180
// Seen from closer to a gate's line than this, the light is taken to be seen
// from this far back, so that the part seen through it grows smoothly to all
// of what is beyond it as the lamp reaches the line.
const EYE_BACK = 0.5
// Further than anything in the arena.
const FAR = 10000
const EVERYWHERE: Polygon = [
  [-FAR, -FAR],
  [FAR, -FAR],
  [FAR, FAR],
  [-FAR, FAR],
]

// One side of a portal or an edge of the arena, from `a` to `b`, which going
// into from the `normal` side of comes out `shift` away. A portal has a side
// facing each way, both on its line.
export type Gate = { a: Point; b: Point; normal: Point; shift: Point; wrap: boolean }

// Where something is seen: the part of the arena, less the parts of it seen
// through gates instead.
export type Seen = { region: Polygon; holes: Polygon[] }

export type View = {
  // How far this copy is from the lightcycle. The lightcycle itself is the
  // first view, with no offset.
  offset: Point
  // Where its headlight and its sprite are seen, or null where not at all.
  beam: Seen | null
  sprite: Seen | null
  // Whether it is seen only through edges of the arena, not portals. Only
  // these copies show the lightcycle's name and emotes: through a portal,
  // they stay with the lightcycle.
  wrapOnly: boolean
}

// The size of a lightcycle, in arena units: how far its lamp is ahead of its
// middle, and half its sprite's length and width.
export type Build = { lamp: number; halfLength: number; halfWidth: number }

// The size of a lightcycle in pixels, as drawn (see Lightcycle.module.css):
// the lamp at the front of the 20px sprite, and the bike about 8px wide.
export const LIGHTCYCLE_BUILD_PX: Build = { lamp: 9, halfLength: 10, halfWidth: 4 }

// Every gate, from the round's portals and wrapping edges.
export function gatesToSee({ wrap, portals }: Gates): Gate[] {
  const gates: Gate[] = []
  if (wrap) {
    const [width, height] = wrap
    const edge = (a: Point, b: Point, normal: Point, shift: Point) => gates.push({ a, b, normal, shift, wrap: true })
    edge([width, 0], [width, height], [-1, 0], [-width, 0])
    edge([0, 0], [0, height], [1, 0], [width, 0])
    edge([0, height], [width, height], [0, -1], [0, -height])
    edge([0, 0], [width, 0], [0, 1], [0, height])
  }
  for (const [[start, end], [outStart]] of portals.flatMap(([first, second]) => [
    [first, second],
    [second, first],
  ])) {
    const shift: Point = [outStart[0] - start[0], outStart[1] - start[1]]
    const across: Point = start[0] === end[0] ? [1, 0] : [0, 1]
    gates.push({ a: start, b: end, normal: across, shift, wrap: false })
    gates.push({ a: start, b: end, normal: scale(across, -1), shift, wrap: false })
  }
  return gates
}

// The lightcycle at `center` going `direction`, built as `build`, and its
// copies, the lightcycle first.
export function viewsOf(center: Point, direction: Direction, build: Build, gates: Gate[]): View[] {
  const heading = velocities[direction]
  const views: View[] = [
    {
      offset: [0, 0],
      beam: { region: EVERYWHERE, holes: [] },
      sprite: { region: EVERYWHERE, holes: [] },
      wrapOnly: true,
    },
  ]
  for (let index = 0; index < views.length; index++) {
    const view = views[index]
    const at = add(center, view.offset)
    const lamp = add(at, scale(heading, build.lamp))
    const beam = beamFrom(lamp, heading)
    const sprite = spriteBox(at, heading, build)
    for (const gate of gates) {
      if (views.length > MAX_COPIES) return views
      if (distanceToSegment(at, gate.a, gate.b) > VIEW_REACH) continue

      // What of the headlight's light and the sprite goes through the gate,
      // if any of them does. Each only reaches the gate across the part of
      // the arena this view sees it in: a copy that has just come out of a
      // portal is behind the twin, but none of it is seen on that side.
      const near = nearSide(gate)
      const light = view.beam && lightThrough(gate, lamp, at, heading)
      const lit = light && view.beam && clip(light, view.beam.region)
      // Once the lamp has gone through ahead of the rest of the lightcycle,
      // it is the lightcycle that has to be seen short of the gate.
      const lampThrough = dot(sub(lamp, gate.a), gate.normal) <= 0
      const lightReaches =
        view.beam &&
        (lampThrough ? contains(view.beam.region, at) : area(clip(clip(beam, near), view.beam.region)) > 0.01)
      const seenLight = lit && lightReaches && area(lit) > 0.01 && area(clip(beam, lit)) > 0.01 ? lit : null
      const band = view.sprite && spriteThrough(gate, at, heading)
      const banded = band && view.sprite && clip(band, view.sprite.region)
      const spriteReaches = view.sprite && area(clip(clip(sprite, near), view.sprite.region)) > 0.01
      const seenSprite =
        banded && spriteReaches && area(banded) > 0.01 && area(clip(sprite, banded)) > 0.01 ? banded : null
      if (!seenLight && !seenSprite) continue

      if (seenLight) view.beam?.holes.push(seenLight)
      if (seenSprite) view.sprite?.holes.push(seenSprite)
      views.push({
        offset: add(view.offset, gate.shift),
        beam: seenLight && { region: move(seenLight, gate.shift), holes: [] },
        sprite: seenSprite && { region: move(seenSprite, gate.shift), holes: [] },
        wrapOnly: view.wrapOnly && gate.wrap,
      })
    }
  }
  return views
}

// The part of the arena the headlight's light reaches through the gate: all
// that is seen from the lamp through it, or once the lamp has gone through
// ahead of the rest of the lightcycle, all that is beyond it.
function lightThrough(gate: Gate, lamp: Point, center: Point, heading: Point): Polygon | null {
  const { a, b, normal } = gate
  const out = dot(sub(lamp, a), normal)
  if (out > 0) {
    const eye = out < EYE_BACK ? add(lamp, scale(normal, EYE_BACK - out)) : lamp
    return beyond(eye, a, b)
  }
  const goingIn = dot(heading, normal) < 0 && dot(sub(center, a), normal) > 0
  return goingIn && level(lamp, gate) ? farSide(gate, false) : null
}

// The part of the arena the sprite reaches through the gate, when it is on
// the gate's side or has just come out of it on that side: beyond the gate's
// line, level with the gate.
function spriteThrough(gate: Gate, center: Point, heading: Point): Polygon | null {
  const out = dot(sub(center, gate.a), gate.normal)
  const onSide = out > 0 || (out === 0 && dot(heading, gate.normal) > 0)
  return onSide ? farSide(gate, true) : null
}

// Everything beyond the gate's line, or only what is level with the gate.
function farSide({ a, b, normal }: Gate, level: boolean): Polygon {
  const along = unit(sub(b, a))
  const [start, end] = level ? [a, b] : [sub(a, scale(along, FAR)), add(b, scale(along, FAR))]
  const away = scale(normal, -FAR)
  return [start, end, add(end, away), add(start, away)]
}

// Everything on the gate's side of its line.
function nearSide({ a, b, normal }: Gate): Polygon {
  const along = scale(unit(sub(b, a)), FAR)
  const [start, end] = [sub(a, along), add(b, along)]
  const toward = scale(normal, FAR)
  return [start, end, add(end, toward), add(start, toward)]
}

// Whether `point` is level with some part of the gate.
function level(point: Point, { a, b }: Gate) {
  const along = a[0] === b[0] ? 1 : 0
  return Math.min(a[along], b[along]) < point[along] && point[along] < Math.max(a[along], b[along])
}

// The part of the arena a headlight at `lamp` going `heading` lights.
function beamFrom(lamp: Point, [x, y]: Point): Polygon {
  const ray = (angle: number): Point => [
    lamp[0] + (x * Math.cos(angle) - y * Math.sin(angle)) * VIEW_REACH,
    lamp[1] + (x * Math.sin(angle) + y * Math.cos(angle)) * VIEW_REACH,
  ]
  return [lamp, ray(-BEAM_SPREAD), ray(0), ray(BEAM_SPREAD)]
}

// The part of the arena covered by the sprite of a lightcycle at `center`
// going `heading`.
export function spriteBox(center: Point, heading: Point, { halfLength, halfWidth }: Build): Polygon {
  const along = scale(heading, halfLength)
  const across = scale([-heading[1], heading[0]], halfWidth)
  return [
    add(add(center, along), across),
    sub(add(center, along), across),
    sub(sub(center, along), across),
    add(sub(center, along), across),
  ]
}

// The part of the arena seen from `eye` through the segment from `a` to `b`:
// a wedge from the segment out to FAR. Its far side follows an arc in steps
// of no more than a twelfth of a half turn, since a wedge seen from right up
// against the segment is nearly a half turn wide, and a straight far side
// would cut all but a sliver of it off.
function beyond(eye: Point, a: Point, b: Point): Polygon {
  const toA = sub(a, eye)
  const toB = sub(b, eye)
  const from = Math.atan2(toB[1], toB[0])
  const turn = Math.atan2(cross(toB, toA), dot(toB, toA))
  const steps = Math.max(1, Math.ceil(Math.abs(turn) / (Math.PI / 12)))
  const arc = Array.from({ length: steps + 1 }, (_, step): Point => {
    const angle = from + (turn * step) / steps
    return add(eye, [Math.cos(angle) * FAR, Math.sin(angle) * FAR])
  })
  return [a, b, ...arc]
}

// A pool of light round `center`, seen only in `region`, or everywhere if
// that is null, less `holes`.
export type Light = { center: Point; region: Polygon | null; holes: Polygon[] }

// The pool of light `reach` across round a lightcycle at `center`, and the
// copies of it seen through gates. Like the headlight (see viewsOf), what is
// seen from `center` through a gate within reach, on the side facing the
// pool, is cut out of the pool, and a copy of the pool moved by the gate's
// shift is seen there instead, moved the same way. Copies go through gates
// in turn.
export function lightsOf(center: Point, reach: number, gates: Gate[]): Light[] {
  const lights: Light[] = [{ center, region: null, holes: [] }]
  for (let index = 0; index < lights.length; index++) {
    const light = lights[index]
    for (const gate of gates) {
      if (lights.length > MAX_COPIES) return lights
      const { a, b, normal, shift } = gate
      if (dot(sub(light.center, a), normal) <= 0) continue
      if (distanceToSegment(light.center, a, b) > reach) continue
      // The light only gets to the gate across the part of the arena it is
      // seen in: a copy that has just come out of a portal is behind the
      // twin, and none of it is seen on that side.
      const approach: Polygon = [a, b, add(b, normal), add(a, normal)]
      if (light.region && area(clip(approach, light.region)) < 0.01) continue
      const through = beyond(light.center, a, b)
      const seen = light.region ? clip(through, light.region) : through
      if (area(seen) < 0.01) continue
      light.holes.push(seen)
      lights.push({ center: add(light.center, shift), region: move(seen, shift), holes: [] })
    }
  }
  return lights
}

// Where the sprite of a lightcycle at `center` going `heading` touches a
// portal: the stretch of the portal it overlaps, and the same stretch of its
// twin.
export type Touch = { at: Point; twin: Point; length: number; upright: boolean }

export function touchOf(center: Point, heading: Point, build: Build, gates: Gate[]): Touch | null {
  const box = spriteBox(center, heading, build)
  const xs = box.map(([x]) => x)
  const ys = box.map(([, y]) => y)
  const [minX, maxX, minY, maxY] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)]
  for (const { a, b, shift, wrap } of gates) {
    if (wrap) continue
    const upright = a[0] === b[0]
    const [line, low, high] = upright ? [a[0], minY, maxY] : [a[1], minX, maxX]
    const [across, from, to] = upright ? [[minX, maxX], a[1], b[1]] : [[minY, maxY], a[0], b[0]]
    if (line < across[0] || line > across[1]) continue
    const start = Math.max(low, Math.min(from, to))
    const end = Math.min(high, Math.max(from, to))
    if (start >= end) continue
    const middle = (start + end) / 2
    const at: Point = upright ? [line, middle] : [middle, line]
    return { at, twin: add(at, shift), length: end - start, upright }
  }
  return null
}

// Where a trail went through a portal and came out of its twin: the end of
// one of its paths, on a portal, and the start of the next, on the twin.
export type Crossing = { key: string; player: string; entry: Point; exit: Point; upright: boolean }

// Every place a trail goes through a portal, which stays open while the
// trail keeps both sides of it.
export function crossingsOf(lightribbons: Record<string, Lightribbon>, gates: Gate[]): Crossing[] {
  const portalGates = gates.filter((gate) => !gate.wrap)
  const crossings: Crossing[] = []
  if (portalGates.length === 0) return crossings
  for (const [player, { fixed }] of Object.entries(lightribbons)) {
    for (let index = 0; index + 1 < fixed.length; index++) {
      const entry = fixed[index][fixed[index].length - 1]
      const exit = fixed[index + 1][0]
      if (!entry || !exit) continue
      const gate = portalGates.find(
        ({ a, b, shift }) =>
          onSegment(entry, a, b) &&
          Math.abs(entry[0] + shift[0] - exit[0]) < 1e-6 &&
          Math.abs(entry[1] + shift[1] - exit[1]) < 1e-6,
      )
      if (!gate) continue
      crossings.push({
        key: `${player}@${entry[0].toFixed(2)},${entry[1].toFixed(2)}`,
        player,
        entry,
        exit,
        upright: gate.a[0] === gate.b[0],
      })
    }
  }
  return crossings
}

// The part of convex polygon `subject` inside convex polygon `clipper`, which
// is empty if they don't overlap, or if `clipper` is.
function clip(subject: Polygon, clipper: Polygon): Polygon {
  if (clipper.length < 3) return []
  let output = counterClockwise(subject)
  const edges = counterClockwise(clipper)
  for (let i = 0; i < edges.length && output.length > 0; i++) {
    const from = edges[i]
    const to = edges[(i + 1) % edges.length]
    const inside = (point: Point) => cross(sub(to, from), sub(point, from)) >= 0
    const input = output
    output = []
    for (let j = 0; j < input.length; j++) {
      const current = input[j]
      const previous = input[(j + input.length - 1) % input.length]
      if (inside(current)) {
        if (!inside(previous)) output.push(crossing(previous, current, from, to))
        output.push(current)
      } else if (inside(previous)) {
        output.push(crossing(previous, current, from, to))
      }
    }
  }
  return output
}

// Where the segment from `p` to `q` crosses the line through `a` and `b`.
function crossing(p: Point, q: Point, a: Point, b: Point): Point {
  const edge = sub(b, a)
  const t = cross(edge, sub(a, p)) / cross(edge, sub(q, p))
  return add(p, scale(sub(q, p), t))
}

// Whether convex polygon `polygon` contains `point`.
function contains(polygon: Polygon, point: Point) {
  const edges = counterClockwise(polygon)
  return edges.every((from, i) => cross(sub(edges[(i + 1) % edges.length], from), sub(point, from)) >= 0)
}

function counterClockwise(polygon: Polygon): Polygon {
  return signedArea(polygon) < 0 ? [...polygon].reverse() : polygon
}

function area(polygon: Polygon) {
  return Math.abs(signedArea(polygon))
}

function signedArea(polygon: Polygon) {
  let sum = 0
  for (let i = 0; i < polygon.length; i++) sum += cross(polygon[i], polygon[(i + 1) % polygon.length])
  return sum / 2
}

function distanceToSegment(point: Point, a: Point, b: Point) {
  const edge = sub(b, a)
  const t = Math.max(0, Math.min(1, dot(sub(point, a), edge) / dot(edge, edge)))
  const nearest = add(a, scale(edge, t))
  return Math.hypot(point[0] - nearest[0], point[1] - nearest[1])
}

function move(polygon: Polygon, shift: Point): Polygon {
  return polygon.map((point) => add(point, shift))
}

const add = (a: Point, b: Point): Point => [a[0] + b[0], a[1] + b[1]]
const sub = (a: Point, b: Point): Point => [a[0] - b[0], a[1] - b[1]]
const scale = (a: Point, k: number): Point => [a[0] * k, a[1] * k]
const dot = (a: Point, b: Point) => a[0] * b[0] + a[1] * b[1]
const cross = (a: Point, b: Point) => a[0] * b[1] - a[1] * b[0]
const unit = (a: Point): Point => scale(a, 1 / (Math.hypot(a[0], a[1]) || 1))

function onSegment(point: Point, a: Point, b: Point) {
  const within = (value: number, from: number, to: number) =>
    Math.min(from, to) - 1e-6 <= value && value <= Math.max(from, to) + 1e-6
  return within(point[0], a[0], b[0]) && within(point[1], a[1], b[1])
}
