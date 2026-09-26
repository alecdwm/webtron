import { closeSocket, openSocket, send } from '@/actions/socket'
import type {
  Action,
  ArenaOverview,
  Color,
  Direction,
  Leaderboard,
  ServerArena,
  ServerMessage,
  SocketState,
  Stage,
  Thunk,
} from '@/types'
import { serverNow } from '@/utils/clock'
import defaultPlayer from '@/utils/defaultPlayer'

export const setStage = (stage: Stage): Action => ({ type: 'SET_STAGE', stage })
export const setPlayerName = (name: string): Action => ({ type: 'SET_PLAYER_NAME', name })
export const setPlayerColor = (color: Color): Action => ({ type: 'SET_PLAYER_COLOR', color })

export function setSocketState(socketState: SocketState): Thunk {
  return (dispatch) => {
    dispatch({ type: 'SET_SOCKET_STATE', socketState })
    if (socketState === 'CONNECTING') dispatch(setStage('Connect'))
    if (socketState === 'OPEN') dispatch(getArenaList())
    if (socketState === 'CLOSED') dispatch(setStage('MainMenu'))
  }
}

export function connect(): Thunk {
  return (dispatch, getState) => {
    const opened = openSocket({
      open: () => {
        // Claim the player's name first, so a name already in use is refused
        // before anything else happens.
        send({ ClaimName: defaultPlayer(getState().player).name })
        dispatch(setSocketState('OPEN'))
      },
      message: (data) => dispatch(receiveSocketMessage(data)),
      error: () => dispatch(setSocketState('CLOSING')),
      close: () => dispatch(setSocketState('CLOSED')),
    })
    if (opened) dispatch(setSocketState('CONNECTING'))
  }
}

// Closes the socket and returns to the main menu straight away, without
// waiting for the close to complete.
export function disconnect(): Thunk {
  return (dispatch) => {
    if (closeSocket()) dispatch(setSocketState('CLOSED'))
  }
}

export function getArenaList(): Thunk {
  return () => send({ GetArenaList: null })
}

export function join(arenaId: string | null = null): Thunk {
  return (_, getState) => send({ Join: { player: defaultPlayer(getState().player), arena_id: arenaId } })
}

export function start(): Thunk {
  return () => send({ Start: null })
}

export function turn(direction: Direction): Thunk {
  return () => send({ Turn: direction })
}

function receiveSocketMessage(data: string): Thunk {
  return (dispatch, getState) => {
    let message: ServerMessage
    try {
      message = JSON.parse(data)
    } catch (error) {
      return console.error('Invalid socket message', data, error)
    }
    const receivedAt = serverNow(getState().serverTimeOffset)

    if (message === 'NameInUse') return dispatch(receiveNameInUse())
    if (typeof message !== 'object' || message === null) return console.warn('Unhandled message received', message)
    if ('ServerTime' in message) return dispatch(receiveServerTime(message.ServerTime))
    if ('ArenaList' in message) return dispatch(receiveArenaList(message.ArenaList))
    if ('ArenaJoined' in message) return dispatch(receiveArenaJoined(message.ArenaJoined))
    if ('ArenaState' in message) return dispatch(receiveArenaState(message.ArenaState, receivedAt))
    if ('ArenaStatePatch' in message) {
      return dispatch({ type: 'RECEIVE_ARENA_STATE_PATCH', statePatch: message.ArenaStatePatch, receivedAt })
    }
    if ('Leaderboard' in message) return dispatch(receiveLeaderboard(message.Leaderboard))
    console.warn('Unhandled message received', message)
  }
}

function receiveServerTime(serverTime: string): Thunk {
  return (dispatch) => {
    const time = Date.parse(serverTime)
    if (Number.isNaN(time)) return console.warn('Invalid server time', serverTime)
    dispatch({ type: 'RECEIVE_SERVER_TIME', serverTimeOffset: time - Date.now() })
  }
}

function receiveArenaList(arenaList: ArenaOverview[]): Thunk {
  return (dispatch, getState) => {
    const prevArenaList = getState().arenaList
    dispatch({ type: 'RECEIVE_ARENA_LIST', arenaList })
    if (prevArenaList === null) {
      if (arenaList.length < 1) dispatch(join())
      else dispatch(setStage('ArenaSelect'))
    }
  }
}

function receiveArenaJoined([arenaId, playerId]: [string, string]): Thunk {
  return (dispatch) => {
    dispatch({ type: 'RECEIVE_ARENA_JOINED', arenaId, playerId })
    dispatch(setStage('Arena'))
  }
}

// Another connected player has the player's name. Disconnecting returns to
// the main menu, which says so.
function receiveNameInUse(): Thunk {
  return (dispatch) => {
    dispatch({ type: 'RECEIVE_NAME_IN_USE' })
    dispatch(disconnect())
  }
}

function receiveArenaState(state: ServerArena, receivedAt: number): Action {
  return { type: 'RECEIVE_ARENA_STATE', state, receivedAt }
}

function receiveLeaderboard(leaderboard: Leaderboard): Action {
  return { type: 'RECEIVE_LEADERBOARD', leaderboard }
}
