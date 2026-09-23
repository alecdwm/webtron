import { useEffect, useRef, useState } from 'react'

import { Engine, playCrash, playLose, playPlayerJoin, playPlayerLeave, playTurn, playWin } from '@/audio/sound'
import type { Arena } from '@/types'

// Plays sounds for arena events by diffing each arena state against the
// previous one, and keeps an engine drone running for every live lightcycle.
export default function useArenaSounds(arena: Arena, selfId: string | null) {
  const previousArena = useRef(arena)
  const engines = useRef(new Map<string, Engine>())
  const running = useRoundRunning(arena.started ? arena.started.valueOf() : null)

  const panFor = (id: string, position: [number, number]) =>
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
      } else if (cycle.direction !== old.direction) {
        playTurn(self, pan)
        engines.current.get(id)?.rev()
      }
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
    const active = engines.current
    const alive = new Set<string>()
    if (running) {
      for (const [id, cycle] of Object.entries(arena.lightcycles)) {
        if (cycle.dead) continue
        alive.add(id)
        let engine = active.get(id)
        if (!engine) {
          engine = new Engine(id === selfId, active.size)
          active.set(id, engine)
        }
        // start() is a no-op once running; calling it on every update also
        // starts engines that were created before audio was unlocked.
        engine.setSpeed(cycle.speed)
        engine.start(panFor(id, cycle.position))
        engine.setPan(panFor(id, cycle.position))
      }
    }
    for (const [id, engine] of active) {
      if (alive.has(id)) continue
      engine.stop()
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

// True once the round's start time has passed, until the round ends.
function useRoundRunning(startAt: number | null) {
  const [reachedStart, setReachedStart] = useState<number | null>(null)

  useEffect(() => {
    if (startAt === null) return
    const timeout = window.setTimeout(() => setReachedStart(startAt), Math.max(0, startAt - Date.now()))
    return () => window.clearTimeout(timeout)
  }, [startAt])

  return startAt !== null && reachedStart === startAt
}
