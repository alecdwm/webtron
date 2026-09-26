import type { Arena, ArenaUpdate, Lightcycle, Lightribbon, Point, ServerArena, ServerLightribbon } from '@/types'
import createReducer from '@/utils/createReducer'

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
  }
}

function parseTime(time: string | null): number | null {
  if (time === null) return null
  const parsed = Date.parse(time)
  return Number.isNaN(parsed) ? null : parsed
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

  if ('Start' in update) return { ...arena, started: parseTime(update.Start), holes: [], derezzing: {} }
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

  if ('UpdateLightribbonAppendPoint' in update) {
    const [id, point] = update.UpdateLightribbonAppendPoint
    return updateLightribbon(arena, id, (lightribbon) => appendPoint(lightribbon, point))
  }
  if ('UpdateLightribbonReplaceLatestPoint' in update) {
    const [id, point] = update.UpdateLightribbonReplaceLatestPoint
    return updateLightribbon(arena, id, ({ fixed }) => ({ fixed: fixed.length > 0 ? fixed : [[]], tip: point }))
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

function without<T>(record: Record<string, T>, id: string): Record<string, T> {
  if (!(id in record)) return record
  const { [id]: _removed, ...rest } = record
  return rest
}
