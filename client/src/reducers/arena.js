import { RECEIVE_ARENA_JOINED, RECEIVE_ARENA_STATE, RECEIVE_ARENA_STATE_PATCH } from 'actions'
import dayjs from 'dayjs'
import createReducer from 'utils/createReducer'

const initialState = null

export default createReducer(initialState, {
  [RECEIVE_ARENA_JOINED]: (_, { arenaId }) => ({
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
  }),
  [RECEIVE_ARENA_STATE]: (_, { state }) => ({ ...state }),
  [RECEIVE_ARENA_STATE_PATCH]: (arena, { statePatch = [] }) => statePatch.reduce(updateArena, arena),
})

function updateArena(arena, update) {
  const updateType = typeof update === 'object' ? Object.keys(update).pop() : update
  const updateData = typeof update === 'object' ? update[updateType] : null

  const updateHandlers = {
    AddPlayer: ([playerId, player]) => ({
      ...arena,
      players: {
        ...arena.players,
        [playerId]: player,
      },
    }),
    AddLightcycle: ([playerId, lightcycle]) => ({
      ...arena,
      lightcycles: {
        ...arena.lightcycles,
        [playerId]: lightcycle,
      },
    }),
    AddLightribbon: ([playerId, lightribbon]) => ({
      ...arena,
      lightribbons: {
        ...arena.lightribbons,
        [playerId]: lightribbon,
      },
    }),

    Start: (startAt) => ({
      ...arena,
      started: dayjs(startAt),
    }),
    End: () => ({
      ...arena,
      started: null,
    }),
    SetWinner: (winner) => ({
      ...arena,
      winner,
    }),

    UpdateLightcyclePosition: ([playerId, position]) => ({
      ...arena,
      lightcycles: {
        ...arena.lightcycles,
        [playerId]: {
          ...arena.lightcycles[playerId],
          position,
        },
      },
    }),
    UpdateLightcycleDirection: ([playerId, direction]) => ({
      ...arena,
      lightcycles: {
        ...arena.lightcycles,
        [playerId]: {
          ...arena.lightcycles[playerId],
          direction,
        },
      },
    }),
    UpdateLightcycleApplyDeath: (playerId) => ({
      ...arena,
      lightcycles: {
        ...arena.lightcycles,
        [playerId]: {
          ...arena.lightcycles[playerId],
          dead: true,
        },
      },
    }),

    UpdateLightribbonAppendPoint: ([playerId, point]) => ({
      ...arena,
      lightribbons: {
        ...arena.lightribbons,
        [playerId]: {
          ...arena.lightribbons[playerId],
          points: [...arena.lightribbons[playerId].points, point],
        },
      },
    }),
    UpdateLightribbonReplaceLatestPoint: ([playerId, latestPoint]) => ({
      ...arena,
      lightribbons: {
        ...arena.lightribbons,
        [playerId]: {
          ...arena.lightribbons[playerId],
          points: [...arena.lightribbons[playerId].points.slice(0, -1), latestPoint],
        },
      },
    }),

    RemovePlayer: (playerId) => ({
      ...arena,
      players: Object.fromEntries(Object.entries(arena.players).filter(([id]) => id !== playerId)),
    }),
    RemoveLightcycle: (playerId) => ({
      ...arena,
      lightcycles: Object.fromEntries(Object.entries(arena.lightcycles).filter(([id]) => id !== playerId)),
    }),
    RemoveLightribbon: (playerId) => ({
      ...arena,
      lightribbons: Object.fromEntries(Object.entries(arena.lightribbons).filter(([id]) => id !== playerId)),
    }),
  }

  return updateHandlers[updateType] ? updateHandlers[updateType](updateData) : arena
}
