import {
  createContext,
  type RefObject,
  useCallback,
  useContext,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
} from 'react'

import type { Arena, Lightcycle, Point, Portal } from '@/types'
import { serverNow } from '@/utils/clock'
import { liveSegmentData } from '@/utils/ribbonPath'
import {
  LAYOUT_CELLS,
  PORTAL_ENDS_MS,
  PORTAL_SURFACE_MS,
  advance,
  type Gates,
  gatesOf,
  noGates,
  velocities,
} from '@/utils/round'
import {
  type Build,
  type Crossing,
  LIGHTCYCLE_BUILD_PX,
  crossingsOf,
  gatesToSee,
  type Polygon,
  lightsOf,
  type Seen,
  touchOf,
  type View,
  viewsOf,
} from '@/utils/views'

// Extrapolation stops this long after the server update that last set a
// lightcycle's position, so a lost or very late update leaves the lightcycle
// waiting instead of carrying it far past a turn or crash. Updates run every
// 50ms and arrive with tens of milliseconds of jitter on top of that, which a
// cap close to one update would turn into a stall and a jump forward.
const MAX_EXTRAPOLATION_MS = 150

// How far the fog's pool of light round the player's lightcycle reaches, as
// a fraction of the arena's width (see components/Fog.tsx).
const FOG_POOL_REACH = 0.26

// How long an emote shows for.
export const EMOTE_MS = 2200
// Where an emote's bubble sits, in pixels from the centre of its lightcycle:
// above it, or below it when there is no room above. It floats away from
// the lightcycle as it fades, by EMOTE_FLOAT. In an arena that doesn't wrap
// round, a bubble is kept EMOTE_MARGIN pixels inside the edges.
const EMOTE_ABOVE = 34
const EMOTE_FLOAT = 16
const EMOTE_MARGIN = 4

// Where to draw a lightcycle at server time `now`: its last position from the
// server, carried forward at its speed from the time of the update that set
// it, while the round is running. Measuring from that time rather than from
// when the update arrived keeps an update that arrives late on the same path
// as the rest. A lightcycle carried into a portal, or off an edge of an arena
// that wraps round, is drawn where it comes out, and `jumped` is set.
export function lightcycleTravel(
  lightcycle: Lightcycle,
  started: number | null,
  now: number,
  gates: Gates = noGates,
): { point: Point; jumped: boolean } {
  const { position, direction, speed, dead, updatedAt } = lightcycle
  const still = { point: position, jumped: false }
  if (dead || started === null || now < started) return still
  const elapsedMs = Math.min(now - Math.max(updatedAt, started), MAX_EXTRAPOLATION_MS)
  if (elapsedMs <= 0) return still
  return advance(position, direction, (speed * elapsedMs) / 1000, gates)
}

export function lightcyclePosition(
  lightcycle: Lightcycle,
  started: number | null,
  now: number,
  gates: Gates = noGates,
): Point {
  return lightcycleTravel(lightcycle, started, now, gates).point
}

export type ArenaAnimation = {
  // Each returns a function that stops tracking the element(s).
  trackLightcycle: (id: string, element: HTMLElement) => () => void
  trackLightribbon: (id: string, livePaths: SVGPathElement[]) => () => void
  // Places the element like the lightcycle, `offset` arena widths and
  // heights away from it.
  // Places the element as copy number `index` of the lightcycle, seen
  // through gates (see utils/views.ts), and hides it when the lightcycle has
  // that few copies.
  trackCopy: (id: string, element: HTMLElement, index: number) => () => void
  // Places the element where the lightcycle's sprite touches a portal, or
  // the same place on the twin, sized to the stretch touched, with the class
  // `across` on it for a portal running across; and hides it while the
  // lightcycle touches no portal.
  trackTouch: (id: string, element: HTMLElement, twin: boolean, across: string) => () => void
  // Shapes the band and line drawn by the element (its first and second
  // paths) for portal `portal`, whose twin is `twin`, rippling (see
  // portalSurface). Its surface ripples in from its ends from `appearAt`, in
  // server time, or is already there if that is null.
  trackPortal: (element: SVGSVGElement, portal: Portal, twin: Portal, appearAt: number | null) => () => void
  // Plays the emote bubble's animation from `at`, in server time, so a
  // bubble added part way through carries on from there. A bubble on a copy
  // of a lightcycle only animates; one on the lightcycle itself is also kept
  // inside the arena if it doesn't wrap round.
  trackEmote: (id: string, element: HTMLElement, at: number, ghost: boolean) => () => void
  // Draws the fog's pools of light round the player's lightcycle, and its
  // copies seen through gates, into the element, filled with the gradient
  // with the id `${prefix}-pool`.
  trackFog: (element: SVGGElement, prefix: string) => () => void
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
// its left and bottom, in percent of the arena, and its name and body parts
// clip-paths hiding what is seen through gates; a copy of it gets its
// position and clip-paths showing only what is seen of it through them, or
// is hidden; a trail's live paths get their
// `d` attribute; the fog gets pools of light round the player's; an emote
// bubble gets a --shift to move it sideways inside the arena, and a
// data-below attribute to put it under its lightcycle. It also draws after
// every render of the arena, so new elements are placed before they are
// first painted. `arenaRef` is the arena's element, measured to place emote
// bubbles in pixels.
export default function useArenaAnimation(
  arena: Arena,
  serverTimeOffset: number,
  selfId: string | null,
  arenaRef: RefObject<HTMLElement | null>,
): ArenaAnimation {
  const latest = useRef({ arena, serverTimeOffset, selfId })
  const lightcycles = useRef(new Map<string, HTMLElement>())
  const copies = useRef(new Map<HTMLElement, { id: string; index: number }>())
  const touches = useRef(new Map<HTMLElement, { id: string; twin: boolean; across: string }>())
  const portals = useRef(new Map<SVGSVGElement, { portal: Portal; twin: Portal; appearAt: number | null }>())
  // The places trails go through portals, worked out again only when the
  // trails change, and when each was first seen, in milliseconds.
  const crossings = useRef<{ from: Arena['lightribbons'] | null; crossings: Crossing[] }>({
    from: null,
    crossings: [],
  })
  const crossedAt = useRef(new Map<string, number>())
  const lightribbons = useRef(new Map<string, SVGPathElement[]>())
  const fogs = useRef(new Map<SVGGElement, string>())
  const emotes = useRef(new Map<HTMLElement, TrackedEmote>())
  // The arena's size in pixels.
  const arenaSize = useRef<Point>([0, 0])
  // The last value written to each element, to skip writes that change nothing.
  const written = useRef(new WeakMap<Element, string>())

  const draw = useCallback(() => {
    const { arena, serverTimeOffset, selfId } = latest.current
    const { width, height, started } = arena
    if (width === 0 || height === 0) return
    const now = serverNow(serverTimeOffset)
    const gates = gatesOf(arena.round, width, height)
    const { wrap } = gates

    // Clip paths are in pixels from the lightcycle's anchor.
    const [arenaWidth, arenaHeight] = arenaSize.current
    const scale: Point = [arenaWidth / width, arenaHeight / height]

    // Each lightcycle and its copies, where it is now. The lightcycle's size
    // in arena units depends on how big the arena is drawn.
    const gatesSeen = gatesToSee(gates)
    const build: Build = {
      lamp: LIGHTCYCLE_BUILD_PX.lamp / scale[0],
      halfLength: LIGHTCYCLE_BUILD_PX.halfLength / scale[0],
      halfWidth: LIGHTCYCLE_BUILD_PX.halfWidth / scale[0],
    }
    const views = new Map<string, { at: Point; views: View[] }>()
    const viewsFor = (id: string) => {
      let found = views.get(id)
      const lightcycle = arena.lightcycles[id]
      if (!found && lightcycle) {
        const at = lightcyclePosition(lightcycle, started, now, gates)
        found = { at, views: viewsOf(at, lightcycle.direction, build, lightcycle.dead ? [] : gatesSeen) }
        views.set(id, found)
      }
      return found
    }

    // Writes where a lightcycle or copy goes and how its parts are clipped,
    // unless it's all as it was. A part is clipped even with nothing cut out
    // of it, since taking the clip off and putting it back as the lightcycle
    // goes through a gate can make the headlight flicker. A copy's parts are
    // hidden where it sees nothing of them.
    const place = (element: HTMLElement, at: Point, view: View, copy: boolean) => {
      const left = `${(at[0] / width) * 100}%`
      const bottom = `${(at[1] / height) * 100}%`
      const clipOf = (seen: Seen | null) => (seen === null ? null : clipPath(seen.region, seen.holes, at, scale))
      const beam = clipOf(view.beam)
      const sprite = clipOf(view.sprite)
      const value = `${left} ${bottom} ${beam} ${sprite} ${view.wrapOnly}`
      if (written.current.get(element) === value) return
      written.current.set(element, value)
      element.style.display = ''
      element.style.left = left
      element.style.bottom = bottom
      const part = (name: string) => element.querySelector<HTMLElement>(`:scope > [data-part=${name}]`)
      for (const [name, clip] of [
        ['beam', beam],
        ['sprite', sprite],
      ] as const) {
        const found = part(name)
        if (!found) continue
        found.style.display = clip === null ? 'none' : ''
        found.style.clipPath = clip ?? ''
        found.style.setProperty('-webkit-clip-path', clip ?? '')
      }
      if (copy) {
        for (const name of ['name', 'emote']) {
          const found = part(name)
          if (found) found.style.display = view.wrapOnly ? '' : 'none'
        }
      }
    }

    for (const [id, element] of lightcycles.current) {
      const found = viewsFor(id)
      if (found) place(element, found.at, found.views[0], false)
    }

    for (const [element, { id, index }] of copies.current) {
      const found = viewsFor(id)
      const view = found?.views[index + 1]
      if (!found || !view) {
        if (written.current.get(element) === 'hidden') continue
        written.current.set(element, 'hidden')
        element.style.display = 'none'
        continue
      }
      place(element, [found.at[0] + view.offset[0], found.at[1] + view.offset[1]], view, true)
    }

    for (const [element, { id, twin, across }] of touches.current) {
      const lightcycle = arena.lightcycles[id]
      const found = viewsFor(id)
      const touch =
        lightcycle && found && !lightcycle.dead
          ? touchOf(found.at, velocities[lightcycle.direction], build, gatesSeen)
          : null
      if (!touch) {
        if (written.current.get(element) === 'hidden') continue
        written.current.set(element, 'hidden')
        element.style.display = 'none'
        continue
      }
      const [x, y] = twin ? touch.twin : touch.at
      const length = `${(touch.length * scale[touch.upright ? 1 : 0]).toFixed(1)}px`
      const value = `${x} ${y} ${length} ${touch.upright}`
      if (written.current.get(element) === value) continue
      written.current.set(element, value)
      element.style.display = ''
      element.style.left = `${(x / width) * 100}%`
      element.style.bottom = `${(y / height) * 100}%`
      element.style.width = touch.upright ? '' : length
      element.style.height = touch.upright ? length : ''
      element.classList.toggle(across, !touch.upright)
    }

    if (portals.current.size > 0) {
      if (crossings.current.from !== arena.lightribbons) {
        const found = crossingsOf(arena.lightribbons, gatesSeen)
        const seen = new Set(found.map(({ key }) => key))
        for (const key of crossedAt.current.keys()) if (!seen.has(key)) crossedAt.current.delete(key)
        // Those already there when the arena is first drawn don't ripple
        // out as if just made.
        const firstTime = crossings.current.from === null
        for (const { key } of found)
          if (!crossedAt.current.has(key)) crossedAt.current.set(key, firstTime ? 0 : performance.now())
        crossings.current = { from: arena.lightribbons, crossings: found }
      }
      const riders = Object.values(arena.lightcycles)
        .filter((lightcycle) => !lightcycle.dead && started !== null)
        .map((lightcycle) => lightcyclePosition(lightcycle, started, now, gates))
      const cell = width / LAYOUT_CELLS
      const ends = ({ x, y, length, upright }: Portal): [Point, Point] => [
        [x * cell, y * cell],
        upright ? [x * cell, (y + length) * cell] : [(x + length) * cell, y * cell],
      ]
      for (const [element, { portal, twin, appearAt }] of portals.current) {
        const reveal =
          appearAt === null ? 1 : Math.max(0, Math.min(1, (now - appearAt - PORTAL_ENDS_MS) / PORTAL_SURFACE_MS))
        const [band, core] = portalSurface(
          [ends(portal), ends(twin)],
          scale[0],
          riders,
          crossings.current.crossings,
          crossedAt.current,
          performance.now(),
          reveal,
        )
        const [bandPath, corePath] = element.querySelectorAll('path')
        bandPath?.setAttribute('d', band)
        corePath?.setAttribute('d', core)
      }
    }

    for (const [id, paths] of lightribbons.current) {
      const lightribbon = arena.lightribbons[id]
      if (!lightribbon) continue
      const lightcycle = arena.lightcycles[id]
      // A lightcycle drawn through a portal or wrapped round ahead of the
      // server's update of its trail isn't joined to it across the arena.
      const travel = lightcycle && !lightcycle.dead ? lightcycleTravel(lightcycle, started, now, gates) : null
      const end = travel && !travel.jumped ? travel.point : null
      const d = liveSegmentData(lightribbon.fixed, lightribbon.tip, end, height)
      for (const path of paths) {
        if (written.current.get(path) === d) continue
        written.current.set(path, d)
        path.setAttribute('d', d)
      }
    }

    for (const [element, emote] of emotes.current) {
      if (emote.ghost) continue
      const lightcycle = arena.lightcycles[emote.id]
      if (!lightcycle) continue
      let shift = 0
      let below = false
      // In an arena that wraps round, a bubble goes through the edges like
      // everything else, and shows on the lightcycle's copies beyond them.
      if (!wrap && arenaWidth > 0) {
        const [x, y] = lightcyclePosition(lightcycle, started, now, gates)
        const left = (x / width) * arenaWidth
        const bottom = (y / height) * arenaHeight
        below = bottom + EMOTE_ABOVE + emote.height + EMOTE_FLOAT > arenaHeight - EMOTE_MARGIN
        const half = emote.width / 2
        shift = Math.max(EMOTE_MARGIN + half - left, Math.min(0, arenaWidth - EMOTE_MARGIN - half - left))
      }
      const value = `${shift.toFixed(1)} ${below}`
      if (written.current.get(element) === value) continue
      written.current.set(element, value)
      element.style.setProperty('--shift', `${shift.toFixed(1)}px`)
      element.toggleAttribute('data-below', below)
      if (below !== emote.below) {
        emote.below = below
        ;(emote.animation.effect as KeyframeEffect).setKeyframes(emoteKeyframes(below))
      }
    }

    const self = selfId !== null ? arena.lightcycles[selfId] : undefined
    if (self && fogs.current.size > 0) {
      const reach = FOG_POOL_REACH * width
      const lights = lightsOf(lightcyclePosition(self, started, now, gates), reach, gatesSeen)
      // Circles in arena units, with y going up, in the fog's box one unit
      // across, each masked to where it is seen.
      const shape = (prefix: string) =>
        lights
          .map(({ center: [x, y], region, holes }, index) => {
            const circle = `<circle cx="${x.toFixed(2)}" cy="${y.toFixed(2)}" r="${reach}" fill="url(#${prefix}-pool)"`
            if (!region && holes.length === 0) return `${circle}/>`
            const mask = `${prefix}-light${index}`
            const seen = region
              ? `<polygon fill="#fff" points="${pointsOf(region)}"/>`
              : `<rect fill="#fff" x="${-FAR}" y="${-FAR}" width="${2 * FAR}" height="${2 * FAR}"/>`
            const cut = holes.map((hole) => `<polygon points="${pointsOf(hole)}"/>`).join('')
            return (
              `<mask id="${mask}" maskUnits="userSpaceOnUse" x="${-FAR}" y="${-FAR}" width="${2 * FAR}" height="${2 * FAR}">${seen}${cut}</mask>` +
              `${circle} mask="url(#${mask})"/>`
            )
          })
          .join('')
      const transform = `matrix(${1 / width} 0 0 ${-1 / height} 0 1)`
      for (const [element, prefix] of fogs.current) {
        const value = shape(prefix)
        if (written.current.get(element) === value) continue
        written.current.set(element, value)
        element.setAttribute('transform', transform)
        element.innerHTML = value
      }
    }
  }, [])

  useLayoutEffect(() => {
    latest.current = { arena, serverTimeOffset, selfId }
    draw()
  })

  useEffect(() => {
    const element = arenaRef.current
    if (!element) return
    const observer = new ResizeObserver(() => {
      arenaSize.current = [element.clientWidth, element.clientHeight]
    })
    observer.observe(element)
    arenaSize.current = [element.clientWidth, element.clientHeight]
    return () => observer.disconnect()
  }, [arenaRef])

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
      trackCopy: (id, element, index) => {
        copies.current.set(element, { id, index })
        return () => copies.current.delete(element)
      },
      trackTouch: (id, element, twin, across) => {
        touches.current.set(element, { id, twin, across })
        return () => touches.current.delete(element)
      },
      trackPortal: (element, portal, twin, appearAt) => {
        portals.current.set(element, { portal, twin, appearAt })
        return () => portals.current.delete(element)
      },
      trackEmote: (id, element, at, ghost) => {
        const animation = element.animate(emoteKeyframes(false), { duration: EMOTE_MS, fill: 'both' })
        animation.currentTime = Math.max(0, serverNow(latest.current.serverTimeOffset) - at)
        emotes.current.set(element, {
          id,
          ghost,
          animation,
          width: element.offsetWidth,
          height: element.offsetHeight,
          below: false,
        })
        return () => {
          animation.cancel()
          emotes.current.delete(element)
        }
      },
      trackFog: (element, prefix) => {
        fogs.current.set(element, prefix)
        return () => fogs.current.delete(element)
      },
    }),
    [],
  )
}

// How wide a portal's surface is drawn, in pixels (see .portal in
// components/ArenaLayout.module.css), and how wide its band is at rest and at
// most either side of its line.
export const PORTAL_WIDTH_PX = 24
const BAND_REST_PX = 4
const BAND_MIN_PX = 2
const BAND_MAX_PX = 6.5
// How far from a portal a lightcycle stirs it, in pixels.
const STIR_REACH_PX = 70

// The band and line of a portal, as paths across PORTAL_WIDTH_PX and 100
// units along it, from its start at the bottom. `lines` are the portal and
// its twin, each from its start to its end, which ripple as one surface.
// Slow waves drift along it both ways; it ripples round the nearest point to
// each lightcycle near either twin, the nearer the stronger; it ripples
// gently where each trail goes through; and a ring of ripple runs out along
// it from where a trail has just gone through. Only `reveal` of it is drawn,
// that fraction of the way in from each end to the middle, with a bulge where
// it is coming in. With reduced motion only the ripples round lightcycles and
// trails are drawn, and they hold still.
function portalSurface(
  lines: [Point, Point][],
  scale: number,
  riders: Point[],
  crossings: Crossing[],
  crossedAt: Map<string, number>,
  nowMs: number,
  reveal: number,
): [string, string] {
  const still = matchMedia('(prefers-reduced-motion: reduce)').matches
  const t = still ? 0 : nowMs / 1000
  if (still) reveal = 1
  const [start, end] = lines[0]
  const length = Math.hypot(end[0] - start[0], end[1] - start[1])
  const lengthPx = length * scale
  // Where a point is along a line, and how far from it, in pixels.
  const place = ([from, to]: [Point, Point], [x, y]: Point) => {
    const unit: Point = [(to[0] - from[0]) / length, (to[1] - from[1]) / length]
    const offset = (x - from[0]) * unit[0] + (y - from[1]) * unit[1]
    const nearest = Math.max(0, Math.min(length, offset))
    const [nx, ny] = [from[0] + unit[0] * nearest, from[1] + unit[1] * nearest]
    return { at: offset * scale, away: Math.hypot(x - nx, y - ny) * scale }
  }
  const stirs = lines
    .flatMap((line) => riders.map((rider) => place(line, rider)))
    .filter(({ away }) => away < STIR_REACH_PX)
  // A trail through the portal is on both twins, at the same place along
  // them, so each is marked once.
  const marks = crossings.flatMap(({ key, entry }) => {
    const on = lines.map((line) => place(line, entry)).find(({ away }) => away < 0.5)
    return on ? [{ at: on.at, age: (nowMs - (crossedAt.get(key) ?? 0)) / 1000 }] : []
  })
  const front = (reveal * lengthPx) / 2

  const ripple = (distance: number, wavelength: number, speed: number) =>
    Math.cos(2 * Math.PI * (distance / wavelength - speed * t))
  const shape = (atPx: number) => {
    let swell = still
      ? 0
      : 0.8 * Math.sin(2 * Math.PI * (atPx / 48 - 0.25 * t)) +
        0.5 * Math.sin(2 * Math.PI * (atPx / 29 + 0.35 * t) + 1.3)
    let sway = still ? 0 : 0.6 * Math.sin(2 * Math.PI * (atPx / 37 - 0.2 * t) + 0.7)
    for (const { at, away } of stirs) {
      const distance = Math.abs(atPx - at)
      const strength = 2 * (1 - away / STIR_REACH_PX) ** 2 * Math.exp(-((distance / 24) ** 2))
      swell += strength * ripple(distance, 16, 0.9)
      sway += 0.5 * strength * ripple(distance, 16, 0.9)
    }
    for (const { at, age } of marks) {
      const distance = Math.abs(atPx - at)
      swell += 0.9 * Math.exp(-((distance / 18) ** 2)) * ripple(distance, 12, 0.6)
      if (!still && age < 1.5) {
        const ring = 60 * age
        swell += 2.2 * Math.exp(-age / 0.5) * Math.exp(-(((distance - ring) / 7) ** 2))
      }
    }
    // Coming in from the ends: a bulge where it has reached, fading in.
    const fromEnd = Math.min(atPx, lengthPx - atPx)
    const shown = reveal >= 1 ? 1 : Math.max(0, Math.min(1, (front - fromEnd) / 8))
    if (reveal < 1) swell += 2.5 * Math.exp(-(((fromEnd - front) / 6) ** 2))
    const half = shown * Math.max(BAND_MIN_PX, Math.min(BAND_MAX_PX, BAND_REST_PX + swell))
    return { half, sway: Math.max(-2, Math.min(2, sway)), shown }
  }

  const steps = Math.max(8, Math.min(120, Math.round(lengthPx / 3)))
  const middle = PORTAL_WIDTH_PX / 2
  const left: string[] = []
  const right: string[] = []
  // The line in pieces, leaving out where the surface isn't in yet.
  const line: string[] = []
  let drawing = false
  for (let step = 0; step <= steps; step++) {
    const fraction = step / steps
    const { half, sway, shown } = shape(fraction * lengthPx)
    const y = (100 * (1 - fraction)).toFixed(2)
    left.push(`${(middle + sway - half).toFixed(2)} ${y}`)
    right.push(`${(middle + sway + half).toFixed(2)} ${y}`)
    if (shown > 0.5) {
      line.push(`${drawing ? 'L' : 'M'}${(middle + sway).toFixed(2)} ${y}`)
      drawing = true
    } else {
      drawing = false
    }
  }
  return [`M${left.join('L')}L${right.reverse().join('L')}Z`, line.join('')]
}

// How big the boxes the headlight and the sprite are clipped in are, in
// pixels, centred on the lightcycle (see .bodyPart in
// components/Lightcycle.module.css): big enough for the beam at full boost
// in Overdrive.
const PART_SIZE = 600

// A clip-path showing `region` less `holes`, for a box PART_SIZE across
// centred on the arena point `from`, `scale` pixels to an arena unit, with y
// going down the screen. The holes are cut out by going round them the other
// way from the region, rather than by the even-odd rule, which not every
// browser takes in a clip-path.
function clipPath(region: Polygon, holes: Polygon[], [fromX, fromY]: Point, [scaleX, scaleY]: Point) {
  const middle = PART_SIZE / 2
  const onScreen = (polygon: Polygon): Polygon =>
    polygon.map(([x, y]) => [(x - fromX) * scaleX + middle, (fromY - y) * scaleY + middle])
  const path = (polygon: Polygon, clockwise: boolean) => {
    const points = onScreen(polygon)
    const turn = points.reduce((sum, [x, y], i) => {
      const [nextX, nextY] = points[(i + 1) % points.length]
      return sum + x * nextY - nextX * y
    }, 0)
    const ordered = turn > 0 === clockwise ? points : [...points].reverse()
    return `M${ordered.map(([x, y]) => `${x.toFixed(1)} ${y.toFixed(1)}`).join('L')}Z`
  }
  return `path('${path(region, true)}${holes.map((hole) => path(hole, false)).join('')}')`
}

// Further than anything in the arena, in arena units.
const FAR = 10000

// A polygon's points for an SVG polygon's points attribute.
function pointsOf(polygon: Polygon) {
  return polygon.map(([x, y]) => `${x.toFixed(2)},${y.toFixed(2)}`).join(' ')
}

type TrackedEmote = {
  id: string
  ghost: boolean
  animation: Animation
  // The bubble's size in pixels, unscaled.
  width: number
  height: number
  below: boolean
}

const EMOTE_EASING = 'cubic-bezier(0.2, 0.8, 0.2, 1)'

// An emote bubble pops up, then floats away from its lightcycle and fades:
// up when it's above the lightcycle, down when it's below. With reduced
// motion it only fades in and out.
function emoteKeyframes(below: boolean): Keyframe[] {
  const away = below ? -1 : 1
  const still = matchMedia('(prefers-reduced-motion: reduce)').matches
  const at = (y: number, scale: number) =>
    still ? 'translate(-50%, 0)' : `translate(-50%, ${-y * away}px) scale(${scale})`
  return [
    { offset: 0, opacity: 0, transform: at(-8, 0.4), easing: EMOTE_EASING },
    { offset: 0.12, opacity: 1, transform: at(0, 1.1), easing: EMOTE_EASING },
    { offset: 0.2, transform: at(0, 1), easing: EMOTE_EASING },
    { offset: 0.8, opacity: 1, transform: at(6, 1), easing: EMOTE_EASING },
    { offset: 1, opacity: 0, transform: at(EMOTE_FLOAT, 0.9) },
  ]
}

function track<T>(elements: Map<string, T>, id: string, element: T) {
  elements.set(id, element)
  return () => {
    if (elements.get(id) === element) elements.delete(id)
  }
}
