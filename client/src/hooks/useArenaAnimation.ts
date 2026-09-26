import { createContext, useCallback, useContext, useEffect, useLayoutEffect, useMemo, useRef } from 'react'

import type { Arena, Direction, Lightcycle, Point } from '@/types'
import { serverNow } from '@/utils/clock'
import { liveSegmentData } from '@/utils/ribbonPath'

// Extrapolation stops this long after a lightcycle's last position update,
// a little over one server tick (50ms), so a late update leaves the
// lightcycle waiting instead of carrying it past a turn or crash.
const MAX_EXTRAPOLATION_MS = 60

const velocities: Record<Direction, Point> = {
  up: [0, 1],
  down: [0, -1],
  left: [-1, 0],
  right: [1, 0],
}

// Where to draw a lightcycle at server time `now`: its last position from the
// server, carried forward at its speed while the round is running.
export function lightcyclePosition(lightcycle: Lightcycle, started: number | null, now: number): Point {
  const { position, direction, speed, dead, receivedAt } = lightcycle
  if (dead || started === null || now < started) return position
  const elapsedMs = Math.min(now - Math.max(receivedAt, started), MAX_EXTRAPOLATION_MS)
  if (elapsedMs <= 0) return position
  const [dx, dy] = velocities[direction]
  const distance = (speed * elapsedMs) / 1000
  return [position[0] + dx * distance, position[1] + dy * distance]
}

export type ArenaAnimation = {
  // Each returns a function that stops tracking the element(s).
  trackLightcycle: (id: string, element: HTMLElement) => () => void
  trackLightribbon: (id: string, livePaths: SVGPathElement[]) => () => void
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
// `d` attribute. It also draws after every render of the
// arena, so new elements are placed before they are first painted.
export default function useArenaAnimation(arena: Arena, serverTimeOffset: number): ArenaAnimation {
  const latest = useRef({ arena, serverTimeOffset })
  const lightcycles = useRef(new Map<string, HTMLElement>())
  const lightribbons = useRef(new Map<string, SVGPathElement[]>())
  // The last value written to each element, to skip writes that change nothing.
  const written = useRef(new WeakMap<Element, string>())

  const draw = useCallback(() => {
    const { arena, serverTimeOffset } = latest.current
    const { width, height, started } = arena
    if (width === 0 || height === 0) return
    const now = serverNow(serverTimeOffset)

    for (const [id, element] of lightcycles.current) {
      const lightcycle = arena.lightcycles[id]
      if (!lightcycle) continue
      const [x, y] = lightcyclePosition(lightcycle, started, now)
      const left = `${(x / width) * 100}%`
      const bottom = `${(y / height) * 100}%`
      const value = `${left} ${bottom}`
      if (written.current.get(element) === value) continue
      written.current.set(element, value)
      element.style.left = left
      element.style.bottom = bottom
    }

    for (const [id, paths] of lightribbons.current) {
      const lightribbon = arena.lightribbons[id]
      if (!lightribbon) continue
      const lightcycle = arena.lightcycles[id]
      const end = lightcycle && !lightcycle.dead ? lightcyclePosition(lightcycle, started, now) : null
      const d = liveSegmentData(lightribbon.fixed, lightribbon.tip, end, height)
      for (const path of paths) {
        if (written.current.get(path) === d) continue
        written.current.set(path, d)
        path.setAttribute('d', d)
      }
    }
  }, [])

  useLayoutEffect(() => {
    latest.current = { arena, serverTimeOffset }
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
