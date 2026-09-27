import { useEffect, useRef, useState } from 'react'

import {
  ENGINE_REV_UP,
  Engine,
  playBrake,
  playCrash,
  playDerez,
  playEmote,
  playLose,
  playNearMiss,
  playPlayerJoin,
  playPlayerLeave,
  playPortal,
  playPortalSpawn,
  startPortalHum,
  playSpawn,
  playTurn,
  playWin,
} from '@/audio/sound'
import type { Arena, Direction, Lightcycle, Point } from '@/types'
import { serverNow } from '@/utils/clock'
import { LAYOUT_CELLS, gatesOf, portalAppearAt, velocities } from '@/utils/round'
import { spawnDelayMs } from '@/utils/spawn'
import { LIGHTCYCLE_BUILD_PX, crossingsOf, gatesToSee, touchOf } from '@/utils/views'

// Plays sounds for arena events by diffing each arena state against the
// previous one, and keeps an engine drone running for every live lightcycle.
export default function useArenaSounds(arena: Arena, selfId: string | null, serverTimeOffset: number) {
  const previousArena = useRef(arena)
  const engines = useRef(new Map<string, Engine>())
  const selfCrashedAt = useRef<number | null>(null)
  // The hum of each lightcycle touching a portal, which stops it.
  const hums = useRef(new Map<string, () => void>())
  // Engines are started ENGINE_REV_UP seconds before the round, and time
  // their power-ups to come on just before it begins.
  const revving = useReachedTime(arena.started === null ? null : arena.started - ENGINE_REV_UP * 1000, serverTimeOffset)

  const panFor = (id: string, position: Point) =>
    id === selfId || !arena.width ? 0 : ((position[0] / arena.width) * 2 - 1) * 0.8

  useEffect(() => {
    const before = previousArena.current
    previousArena.current = arena
    // Width is 0 until the first full arena state arrives. Skip that first
    // diff, or joining an arena would play a join sound for everyone in it.
    if (before === arena || before.width === 0) return

    for (const [id, cycle] of Object.entries(arena.lightcycles)) {
      const old = before.lightcycles[id]
      if (!old || old.dead) continue
      const self = id === selfId
      const pan = panFor(id, cycle.position)
      if (cycle.dead) {
        playCrash(self, pan)
        continue
      }
      if (cycle.direction !== old.direction) {
        playTurn(self, pan)
        engines.current.get(id)?.rev()
      }
      if (cycle.braking && cycle.brake > 0 && !(old.braking && old.brake > 0)) playBrake(self, pan)
    }

    // A lightcycle going through a portal opens a new window in it.
    if (arena.round.portals.length > 0) {
      const gates = gatesToSee(gatesOf(arena.round, arena.width, arena.height))
      const seen = new Set(crossingsOf(before.lightribbons, gates).map(({ key }) => key))
      for (const { key, player, exit } of crossingsOf(arena.lightribbons, gates)) {
        if (!seen.has(key) && arena.lightcycles[player] && !arena.lightcycles[player].dead) {
          playPortal(player === selfId, panFor(player, exit))
        }
      }
    }

    const seenNearMisses = new Set(before.nearMisses.map(({ key }) => key))
    for (const { key, player, point, perfect } of arena.nearMisses) {
      if (!seenNearMisses.has(key)) playNearMiss(player === selfId, perfect, panFor(player, point))
    }

    for (const [id, { key }] of Object.entries(arena.emotes)) {
      if (before.emotes[id]?.key === key) continue
      const cycle = arena.lightcycles[id]
      playEmote(cycle ? panFor(id, cycle.position) : 0)
    }

    // Every lightcycle is placed afresh when a round's countdown begins, and
    // then the portals appear one by one.
    if (arena.started !== null && arena.started !== before.started) {
      Object.entries(arena.lightcycles).forEach(([id, cycle], index) =>
        playSpawn(id === selfId, panFor(id, cycle.position), spawnDelayMs(index) / 1000),
      )
      const now = serverNow(serverTimeOffset)
      arena.round.portals
        .flatMap(({ twins }) => twins)
        .forEach(({ x, length, upright }, index) => {
          const delay = (portalAppearAt(arena.started!, index) - now) / 1000
          const middle = (upright ? x : x + length / 2) / LAYOUT_CELLS
          if (delay >= 0) playPortalSpawn((middle * 2 - 1) * 0.8, delay)
        })
    }

    // A hum for as long as a lightcycle touches a portal. Its size is taken
    // as if the arena were drawn a pixel to a unit, which is about right.
    const build = LIGHTCYCLE_BUILD_PX
    const gates = arena.width === 0 ? [] : gatesToSee(gatesOf(arena.round, arena.width, arena.height))
    for (const [id, stop] of hums.current) {
      const cycle = arena.lightcycles[id]
      if (!cycle || cycle.dead || !touchOf(cycle.position, velocities[cycle.direction], build, gates)) {
        stop()
        hums.current.delete(id)
      }
    }
    for (const [id, cycle] of Object.entries(arena.lightcycles)) {
      if (cycle.dead || hums.current.has(id) || arena.started === null) continue
      if (touchOf(cycle.position, velocities[cycle.direction], build, gates)) {
        hums.current.set(id, startPortalHum(id === selfId, panFor(id, cycle.position)))
      }
    }

    for (const [id, { tip }] of Object.entries(arena.derezzing)) {
      if (id in before.derezzing) continue
      playDerez(tip ? panFor(id, tip) : 0)
    }

    const beforeIds = Object.keys(before.players)
    const ids = Object.keys(arena.players)
    if (ids.some((id) => !(id in before.players))) playPlayerJoin()
    else if (beforeIds.some((id) => !(id in arena.players))) playPlayerLeave()

    if (arena.winner && arena.winner !== before.winner) {
      if (arena.winner === selfId) playWin()
      else playLose()
    }
  })

  useEffect(() => {
    const touching = hums.current
    return () => {
      touching.forEach((stop) => stop())
      touching.clear()
    }
  }, [])

  useEffect(() => {
    const active = engines.current
    const alive = new Set<string>()
    const selfCycle = selfId !== null ? arena.lightcycles[selfId] : undefined
    const listener = selfCycle?.position
    // Once the player's lightcycle crashes, the mix moves from being heard from
    // where it crashed to the even mix a spectator hears, over SPECTATE_BLEND_MS.
    if (selfCycle?.dead) selfCrashedAt.current ??= performance.now()
    else selfCrashedAt.current = null
    const spectate =
      selfCrashedAt.current === null ? 0 : smoothstep((performance.now() - selfCrashedAt.current) / SPECTATE_BLEND_MS)
    if (revving && arena.started !== null) {
      const untilStart = (arena.started - serverNow(serverTimeOffset)) / 1000
      for (const [id, cycle] of Object.entries(arena.lightcycles)) {
        if (cycle.dead) continue
        alive.add(id)
        let engine = active.get(id)
        if (!engine) {
          engine = new Engine(id === selfId, id)
          active.set(id, engine)
        }
        // start() is a no-op once running; calling it on every update also
        // starts engines that were created before audio was unlocked.
        engine.setSpeed(cycle.speed)
        engine.start(panFor(id, cycle.position), untilStart)
        engine.setPan(panFor(id, cycle.position))
        if (listener && id !== selfId) {
          const closeness = proximity(listener, cycle.position, arena.width)
          engine.setProximity(closeness + (1 - closeness) * spectate)
          engine.setDoppler(1 + (doppler(selfCycle, cycle) - 1) * (1 - spectate))
        }
      }
    }
    for (const [id, engine] of active) {
      if (alive.has(id)) continue
      // Engines still running when the round ends wind down; the rest crashed.
      engine.stop(!revving)
      active.delete(id)
    }
  })

  useEffect(() => {
    const active = engines.current
    return () => {
      active.forEach((engine) => engine.stop())
      active.clear()
    }
  }, [])
}

const SPECTATE_BLEND_MS = 3000

function smoothstep(x: number) {
  const t = Math.max(0, Math.min(1, x))
  return t * t * (3 - 2 * t)
}

// The speed of sound for Doppler shifts between lightcycles, in arena units
// per second. It is far slower than real sound relative to the lightcycles,
// so that two passing head-on at full speed bend each other's pitch by about
// three semitones either side of passing.
const DOPPLER_SOUND_SPEED = 600

const directions: Record<Direction, Point> = {
  up: [0, 1],
  down: [0, -1],
  left: [-1, 0],
  right: [1, 0],
}

function velocity({ direction, speed, dead }: Lightcycle): Point {
  if (dead) return [0, 0]
  const [x, y] = directions[direction]
  return [x * speed, y * speed]
}

// The Doppler factor for a lightcycle's engine heard from the listener's.
function doppler(listener: Lightcycle, source: Lightcycle) {
  const dx = source.position[0] - listener.position[0]
  const dy = source.position[1] - listener.position[1]
  const distance = Math.hypot(dx, dy)
  if (distance < 1) return 1
  const [lx, ly] = velocity(listener)
  const [sx, sy] = velocity(source)
  // Positive while they are closing on each other.
  const closing = ((lx - sx) * dx + (ly - sy) * dy) / distance
  return 1 + closing / DOPPLER_SOUND_SPEED
}

// How close a lightcycle is to the listener, from 1 when level with them,
// through 0.5 at 30% of the arena's width away, towards 0.
function proximity(listener: Point, position: Point, arenaWidth: number) {
  const distance = Math.hypot(position[0] - listener[0], position[1] - listener[1]) / (arenaWidth * 0.3)
  return 1 / (1 + distance * distance)
}

// True once server time `at` has passed, until `at` changes or becomes null.
function useReachedTime(at: number | null, serverTimeOffset: number) {
  const [reached, setReached] = useState<number | null>(null)

  useEffect(() => {
    if (at === null) return
    const delay = Math.max(0, at - serverNow(serverTimeOffset))
    const timeout = window.setTimeout(() => setReached(at), delay)
    return () => window.clearTimeout(timeout)
  }, [at, serverTimeOffset])

  return at !== null && reached === at
}
