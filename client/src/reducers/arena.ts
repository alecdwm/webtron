import type { Arena, ArenaUpdate, Lightcycle, Lightribbon, Point, Round, ServerArena, ServerLightribbon } from '@/types'
import createReducer from '@/utils/createReducer'

const emptyRound: Round = { shape: 'Grid', mutators: [], blocked: [] }
// How many recent near misses are kept to be shown.
const MAX_NEAR_MISSES = 12
// Numbers near misses and emotes, so each is shown once.
let nextEffectKey = 0

export default createReducer<Arena | null>(null, {
  RECEIVE_ARENA_JOINED: (_, { arenaId }) => ({
    id: arenaId,
    name: '',
    width: 0,
    height: 0,
    max_players: 0,

    started: null,
    winner: null,

    players: {},
    lightcycles: {},
    lightribbons: {},
    holes: [],
    derezzing: {},
    round: emptyRound,
    previousBlocked: [],
    nearMisses: [],
    emotes: {},
  }),
  RECEIVE_ARENA_STATE: (_, { state, time }) => fromServerArena(state, time),
  RECEIVE_ARENA_STATE_PATCH: (arena, { statePatch, time }) =>
    arena === null ? null : statePatch.reduce((arena, update) => applyUpdate(arena, update, time), arena),
})

function fromServerArena(state: ServerArena, time: number): Arena {
  return {
    id: state.id,
    name: state.name,
    width: state.width,
    height: state.height,
    max_players: state.max_players,

    started: parseTime(state.started),
    winner: state.winner,

    players: state.players,
    lightcycles: Object.fromEntries(
      Object.entries(state.lightcycles).map(([id, lightcycle]) => [id, { ...lightcycle, updatedAt: time }]),
    ),
    lightribbons: Object.fromEntries(
      Object.entries(state.lightribbons).map(([id, lightribbon]) => [id, fromServerLightribbon(lightribbon)]),
    ),
    holes: state.holes,
    derezzing: {},
    round: state.round,
    previousBlocked: state.round.blocked,
    nearMisses: [],
    emotes: {},
  }
}

function parseTime(time: string | null): number | null {
  if (time === null) return null
  const parsed = Date.parse(time)
  return Number.isNaN(parsed) ? null : parsed
}

// Every path of the trail, with the tip as the last point of the last path.
function toPaths({ fixed, tip }: Lightribbon): Point[][] {
  if (tip === null) return fixed
  const last = fixed.length > 0 ? fixed[fixed.length - 1] : []
  return [...fixed.slice(0, -1), [...last, tip]]
}

function fromServerLightribbon({ paths }: ServerLightribbon): Lightribbon {
  const last = paths.length > 0 ? paths[paths.length - 1] : []
  if (last.length === 0) return { fixed: paths, tip: null }
  return { fixed: [...paths.slice(0, -1), last.slice(0, -1)], tip: last[last.length - 1] }
}

// Updates for players, lightcycles or trails the client doesn't have are
// ignored, as the server does.
function applyUpdate(arena: Arena, update: ArenaUpdate, time: number): Arena {
  if (update === 'End') return { ...arena, started: null }

  if ('AddPlayer' in update) {
    const [id, player] = update.AddPlayer
    return { ...arena, players: { ...arena.players, [id]: player } }
  }
  if ('AddLightcycle' in update) {
    const [id, lightcycle] = update.AddLightcycle
    return { ...arena, lightcycles: { ...arena.lightcycles, [id]: { ...lightcycle, updatedAt: time } } }
  }
  if ('AddLightribbon' in update) {
    const [id, lightribbon] = update.AddLightribbon
    return { ...arena, lightribbons: { ...arena.lightribbons, [id]: fromServerLightribbon(lightribbon) } }
  }
  if ('AddHole' in update) return { ...arena, holes: [...arena.holes, update.AddHole] }

  if ('SetRound' in update) return { ...arena, round: update.SetRound, previousBlocked: arena.round.blocked }
  if ('Start' in update) {
    return { ...arena, started: parseTime(update.Start), holes: [], derezzing: {}, nearMisses: [] }
  }
  if ('SetWinner' in update) return { ...arena, winner: update.SetWinner }

  if ('UpdateLightcyclePosition' in update) {
    const [id, position] = update.UpdateLightcyclePosition
    return updateLightcycle(arena, id, { position, updatedAt: time })
  }
  if ('UpdateLightcycleDirection' in update) {
    const [id, direction] = update.UpdateLightcycleDirection
    return updateLightcycle(arena, id, { direction })
  }
  if ('UpdateLightcycleSpeed' in update) {
    const [id, speed] = update.UpdateLightcycleSpeed
    return updateLightcycle(arena, id, { speed })
  }
  if ('UpdateLightcycleApplyDeath' in update) {
    const [id, deathCause] = update.UpdateLightcycleApplyDeath
    return updateLightcycle(arena, id, { dead: true, death_cause: deathCause })
  }
  if ('UpdateLightcycleBraking' in update) {
    const [id, braking] = update.UpdateLightcycleBraking
    return updateLightcycle(arena, id, { braking })
  }
  if ('UpdateLightcycleBrakeMeter' in update) {
    const [id, brake] = update.UpdateLightcycleBrakeMeter
    return updateLightcycle(arena, id, { brake })
  }

  if ('UpdateLightribbonAppendPoint' in update) {
    const [id, point] = update.UpdateLightribbonAppendPoint
    return updateLightribbon(arena, id, (lightribbon) => appendPoint(lightribbon, point))
  }
  if ('UpdateLightribbonReplaceLatestPoint' in update) {
    const [id, point] = update.UpdateLightribbonReplaceLatestPoint
    return updateLightribbon(arena, id, ({ fixed }) => ({ fixed: fixed.length > 0 ? fixed : [[]], tip: point }))
  }
  if ('UpdateLightribbonStartPath' in update) {
    const [id, point] = update.UpdateLightribbonStartPath
    return updateLightribbon(arena, id, (lightribbon) => ({ fixed: [...toPaths(lightribbon), [point]], tip: point }))
  }
  if ('TrimLightribbon' in update) {
    const [id, length] = update.TrimLightribbon
    return updateLightribbon(arena, id, (lightribbon) =>
      fromServerLightribbon({ paths: trimFront(toPaths(lightribbon), length) }),
    )
  }

  if ('NearMiss' in update) {
    const [player, point] = update.NearMiss
    const nearMiss = { key: nextEffectKey++, player, point }
    return { ...arena, nearMisses: [...arena.nearMisses, nearMiss].slice(-MAX_NEAR_MISSES) }
  }
  if ('Emote' in update) {
    const [player, emote] = update.Emote
    return { ...arena, emotes: { ...arena.emotes, [player]: { key: nextEffectKey++, emote } } }
  }

  if ('RemovePlayer' in update) return { ...arena, players: without(arena.players, update.RemovePlayer) }
  if ('RemoveLightcycle' in update)
    return { ...arena, lightcycles: without(arena.lightcycles, update.RemoveLightcycle) }
  if ('RemoveLightribbon' in update) {
    return { ...arena, lightribbons: without(arena.lightribbons, update.RemoveLightribbon) }
  }
  if ('DerezLightribbon' in update) {
    const id = update.DerezLightribbon
    const lightribbon = arena.lightribbons[id]
    if (!lightribbon) return arena
    return {
      ...arena,
      lightribbons: without(arena.lightribbons, id),
      derezzing: { ...arena.derezzing, [id]: lightribbon },
    }
  }

  return arena
}

function updateLightcycle(arena: Arena, id: string, change: Partial<Lightcycle>): Arena {
  const lightcycle = arena.lightcycles[id]
  if (!lightcycle) return arena
  return { ...arena, lightcycles: { ...arena.lightcycles, [id]: { ...lightcycle, ...change } } }
}

function updateLightribbon(arena: Arena, id: string, update: (lightribbon: Lightribbon) => Lightribbon): Arena {
  const lightribbon = arena.lightribbons[id]
  if (!lightribbon) return arena
  return { ...arena, lightribbons: { ...arena.lightribbons, [id]: update(lightribbon) } }
}

// The tip stays where it is as a fixed point, and the new point becomes the
// tip. Only the last fixed path is copied, so the rest keep their identity.
function appendPoint({ fixed, tip }: Lightribbon, point: Point): Lightribbon {
  if (tip === null) return { fixed: fixed.length > 0 ? fixed : [[]], tip: point }
  const last = fixed.length > 0 ? fixed[fixed.length - 1] : []
  return { fixed: [...fixed.slice(0, -1), [...last, tip]], tip: point }
}

// Removes `length` from the oldest end of the trail, as the server does (see
// Lightribbon::trim_front in src/server/arena/entities.rs).
function trimFront(paths: Point[][], length: number): Point[][] {
  const trimmed = paths.map((path) => [...path])
  while (length > 0 && trimmed.length > 0) {
    const path = trimmed[0]
    if (path.length < 2) break
    const [[fromX, fromY], [toX, toY]] = path
    const segment = Math.hypot(toX - fromX, toY - fromY)
    if (segment > length) {
      const t = length / segment
      path[0] = [fromX + (toX - fromX) * t, fromY + (toY - fromY) * t]
      break
    }
    length -= segment
    path.shift()
    if (path.length < 2) {
      // Nothing left of the only path but the lightcycle's end of it.
      if (trimmed.length === 1) {
        path.unshift(path[0])
        break
      }
      trimmed.shift()
    }
  }
  return trimmed
}

function without<T>(record: Record<string, T>, id: string): Record<string, T> {
  if (!(id in record)) return record
  const { [id]: _removed, ...rest } = record
  return rest
}
