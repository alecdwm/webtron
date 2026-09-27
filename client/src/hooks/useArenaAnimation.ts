import { createContext, useCallback, useContext, useEffect, useLayoutEffect, useMemo, useRef } from 'react'

import type { Arena, Direction, Lightcycle, Point } from '@/types'
import { serverNow } from '@/utils/clock'
import { liveSegmentData } from '@/utils/ribbonPath'

// Extrapolation stops this long after the server update that last set a
// lightcycle's position, so a lost or very late update leaves the lightcycle
// waiting instead of carrying it far past a turn or crash. Updates run every
// 50ms and arrive with tens of milliseconds of jitter on top of that, which a
// cap close to one update would turn into a stall and a jump forward.
const MAX_EXTRAPOLATION_MS = 150

// How far outside the arena, in arena units, a copy of a lightcycle in an
// arena that wraps round is still drawn: as far as its beam can reach.
const GHOST_REACH = 250
// How far outside the arena, in arena units, the fog's pool of light around a
// copy of the player's lightcycle can still light part of it.
const FOG_POOL_REACH = 150

// Where copies of a lightcycle go in an arena that wraps round, in arena
// widths and heights from it: one past each side and corner of the arena.
export const ghostOffsets: Point[] = [-1, 0, 1].flatMap((dx) =>
  [-1, 0, 1].filter((dy) => dx !== 0 || dy !== 0).map((dy): Point => [dx, dy]),
)

const velocities: Record<Direction, Point> = {
  up: [0, 1],
  down: [0, -1],
  left: [-1, 0],
  right: [1, 0],
}

// Where to draw a lightcycle at server time `now`: its last position from the
// server, carried forward at its speed from the time of the update that set
// it, while the round is running. Measuring from that time rather than from
// when the update arrived keeps an update that arrives late on the same path
// as the rest. In an arena that wraps round, `wrap` is its size, and a
// lightcycle carried off one edge is drawn coming in at the opposite one.
export function lightcyclePosition(
  lightcycle: Lightcycle,
  started: number | null,
  now: number,
  wrap: Point | null = null,
): Point {
  const { position, direction, speed, dead, updatedAt } = lightcycle
  if (dead || started === null || now < started) return position
  const elapsedMs = Math.min(now - Math.max(updatedAt, started), MAX_EXTRAPOLATION_MS)
  if (elapsedMs <= 0) return position
  const [dx, dy] = velocities[direction]
  const distance = (speed * elapsedMs) / 1000
  const x = position[0] + dx * distance
  const y = position[1] + dy * distance
  if (!wrap) return [x, y]
  const [width, height] = wrap
  return [((x % width) + width) % width, ((y % height) + height) % height]
}

export type ArenaAnimation = {
  // Each returns a function that stops tracking the element(s).
  trackLightcycle: (id: string, element: HTMLElement) => () => void
  trackLightribbon: (id: string, livePaths: SVGPathElement[]) => () => void
  // Places the element like the lightcycle, `offset` arena widths and
  // heights away from it.
  trackGhost: (id: string, element: HTMLElement, offset: Point) => () => void
  // Cuts pools of light out of the element's mask around the player's
  // lightcycle.
  trackFog: (element: HTMLElement) => () => void
}

export const ArenaAnimationContext = createContext<ArenaAnimation | null>(null)

export function useArenaAnimationContext(): ArenaAnimation {
  const animation = useContext(ArenaAnimationContext)
  if (animation === null) throw new Error('useArenaAnimationContext must be used inside an ArenaAnimationContext')
  return animation
}

// Moves every tracked lightcycle, and the live end of every tracked trail, to
// its extrapolated position once per animation frame, writing to the DOM
// directly so nothing re-renders. A lightcycle element gets its position as
// its left and bottom, in percent of the arena; a trail's live paths get their
// `d` attribute; the fog gets pools of light around the player's. It also draws after
// every render of the arena, so new elements are placed before they are
// first painted.
export default function useArenaAnimation(
  arena: Arena,
  serverTimeOffset: number,
  selfId: string | null,
): ArenaAnimation {
  const latest = useRef({ arena, serverTimeOffset, selfId })
  const lightcycles = useRef(new Map<string, HTMLElement>())
  const lightribbons = useRef(new Map<string, SVGPathElement[]>())
  const fogs = useRef(new Map<string, HTMLElement>())
  const ghosts = useRef(new Map<HTMLElement, { id: string; offset: Point }>())
  // The last value written to each element, to skip writes that change nothing.
  const written = useRef(new WeakMap<Element, string>())

  const draw = useCallback(() => {
    const { arena, serverTimeOffset, selfId } = latest.current
    const { width, height, started } = arena
    if (width === 0 || height === 0) return
    const now = serverNow(serverTimeOffset)
    const wrap: Point | null = arena.round.mutators.includes('Wrap') ? [width, height] : null

    for (const [id, element] of lightcycles.current) {
      const lightcycle = arena.lightcycles[id]
      if (!lightcycle) continue
      const [x, y] = lightcyclePosition(lightcycle, started, now, wrap)
      const left = `${(x / width) * 100}%`
      const bottom = `${(y / height) * 100}%`
      const value = `${left} ${bottom}`
      if (written.current.get(element) === value) continue
      written.current.set(element, value)
      element.style.left = left
      element.style.bottom = bottom
    }

    for (const [element, { id, offset }] of ghosts.current) {
      const lightcycle = arena.lightcycles[id]
      if (!lightcycle) continue
      const [x, y] = lightcyclePosition(lightcycle, started, now, wrap)
      const ghostX = x + offset[0] * width
      const ghostY = y + offset[1] * height
      // Hidden unless near enough for its beam to reach into the arena.
      const near =
        ghostX > -GHOST_REACH && ghostX < width + GHOST_REACH && ghostY > -GHOST_REACH && ghostY < height + GHOST_REACH
      const left = `${(ghostX / width) * 100}%`
      const bottom = `${(ghostY / height) * 100}%`
      const value = near ? `${left} ${bottom}` : 'hidden'
      if (written.current.get(element) === value) continue
      written.current.set(element, value)
      element.style.display = near ? '' : 'none'
      if (!near) continue
      element.style.left = left
      element.style.bottom = bottom
    }

    for (const [id, paths] of lightribbons.current) {
      const lightribbon = arena.lightribbons[id]
      if (!lightribbon) continue
      const lightcycle = arena.lightcycles[id]
      let end = lightcycle && !lightcycle.dead ? lightcyclePosition(lightcycle, started, now, wrap) : null
      // A lightcycle drawn wrapped round ahead of the server's update of its
      // trail isn't joined to it across the arena.
      const tip = lightribbon.tip
      if (end && tip && (Math.abs(end[0] - tip[0]) > width / 2 || Math.abs(end[1] - tip[1]) > height / 2)) end = null
      const d = liveSegmentData(lightribbon.fixed, lightribbon.tip, end, height)
      for (const path of paths) {
        if (written.current.get(path) === d) continue
        written.current.set(path, d)
        path.setAttribute('d', d)
      }
    }

    const self = selfId !== null ? arena.lightcycles[selfId] : undefined
    if (self) {
      const [x, y] = lightcyclePosition(self, started, now, wrap)
      // In an arena that wraps round, the light also comes through the
      // edges the lightcycle is near, from its copies beyond them.
      const pools = [[x, y], ...(wrap ? ghostOffsets.map(([dx, dy]) => [x + dx * width, y + dy * height]) : [])]
        .filter(
          ([poolX, poolY]) =>
            poolX > -FOG_POOL_REACH &&
            poolX < width + FOG_POOL_REACH &&
            poolY > -FOG_POOL_REACH &&
            poolY < height + FOG_POOL_REACH,
        )
        .map(
          ([poolX, poolY]) =>
            `radial-gradient(circle at ${(poolX / width) * 100}% ${(1 - poolY / height) * 100}%, transparent 12cqi, rgba(0, 0, 0, 0.77) 18cqi, #000 26cqi)`,
        )
      const value = pools.join(', ')
      for (const element of fogs.current.values()) {
        if (written.current.get(element) === value) continue
        written.current.set(element, value)
        element.style.maskImage = value
      }
    }
  }, [])

  useLayoutEffect(() => {
    latest.current = { arena, serverTimeOffset, selfId }
    draw()
  })

  useEffect(() => {
    let frame = requestAnimationFrame(function onFrame() {
      draw()
      frame = requestAnimationFrame(onFrame)
    })
    return () => cancelAnimationFrame(frame)
  }, [draw])

  return useMemo(
    () => ({
      trackLightcycle: (id, element) => track(lightcycles.current, id, element),
      trackLightribbon: (id, livePaths) => track(lightribbons.current, id, livePaths),
      trackGhost: (id, element, offset) => {
        ghosts.current.set(element, { id, offset })
        return () => ghosts.current.delete(element)
      },
      trackFog: (element) => track(fogs.current, 'fog', element),
    }),
    [],
  )
}

function track<T>(elements: Map<string, T>, id: string, element: T) {
  elements.set(id, element)
  return () => {
    if (elements.get(id) === element) elements.delete(id)
  }
}
