import { closeSocket, openSocket, send } from '@/actions/socket'
import { forgetSentTurns, trackSentTurns } from '@/actions/steering'
import type {
  Action,
  ArenaOverview,
  Color,
  ControlScheme,
  Direction,
  Emote,
  Leaderboard,
  ServerMessage,
  SocketState,
  Stage,
  Thunk,
} from '@/types'
import { resetServerClock, sampleServerClock } from '@/utils/clock'
import { storeControlScheme } from '@/utils/controlScheme'
import defaultPlayer from '@/utils/defaultPlayer'
import { storePlayerColor, storePlayerName } from '@/utils/playerSettings'

export const setStage = (stage: Stage): Action => ({ type: 'SET_STAGE', stage })

export function setPlayerName(name: string): Thunk {
  return (dispatch) => {
    storePlayerName(name)
    dispatch({ type: 'SET_PLAYER_NAME', name })
  }
}

export function setPlayerColor(color: Color): Thunk {
  return (dispatch) => {
    storePlayerColor(color)
    dispatch({ type: 'SET_PLAYER_COLOR', color })
  }
}

export function setControlScheme(controlScheme: ControlScheme): Thunk {
  return (dispatch) => {
    storeControlScheme(controlScheme)
    dispatch({ type: 'SET_CONTROL_SCHEME', controlScheme })
  }
}

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
        resetServerClock()
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

// Whether the brake was last sent as held, so held keys and several inputs
// don't send it again.
let brakeHeld = false

export function brake(held: boolean): Thunk {
  return () => {
    if (held === brakeHeld) return
    brakeHeld = held
    send({ Brake: held })
  }
}

export function emote(emote: Emote): Thunk {
  return () => send({ Emote: emote })
}

export { steer } from '@/actions/steering'

function receiveSocketMessage(data: string): Thunk {
  return (dispatch, getState) => {
    let message: ServerMessage
    try {
      message = JSON.parse(data)
    } catch (error) {
      return console.error('Invalid socket message', data, error)
    }
    if (message === 'NameInUse') return dispatch(receiveNameInUse())
    if (typeof message !== 'object' || message === null) return console.warn('Unhandled message received', message)
    if ('ArenaList' in message) return dispatch(receiveArenaList(message.ArenaList))
    if ('ArenaJoined' in message) return dispatch(receiveArenaJoined(message.ArenaJoined))
    if ('ArenaState' in message) {
      const { time, state } = message.ArenaState
      dispatch(receiveServerTime(time))
      forgetSentTurns()
      return dispatch({ type: 'RECEIVE_ARENA_STATE', state, time })
    }
    if ('ArenaStatePatch' in message) {
      const { time, updates } = message.ArenaStatePatch
      dispatch(receiveServerTime(time))
      trackSentTurns(updates, getState().player.id)
      return dispatch({ type: 'RECEIVE_ARENA_STATE_PATCH', statePatch: updates, time })
    }
    if ('Leaderboard' in message) return dispatch(receiveLeaderboard(message.Leaderboard))
    console.warn('Unhandled message received', message)
  }
}

// Updates the estimate of the server's clock from a message's stamp. The
// estimate changes only when a sample closer than the others arrives or an old
// one expires, so most messages leave it alone.
function receiveServerTime(time: number): Thunk {
  return (dispatch, getState) => {
    const serverTimeOffset = sampleServerClock(time)
    if (serverTimeOffset !== getState().serverTimeOffset) dispatch({ type: 'RECEIVE_SERVER_TIME', serverTimeOffset })
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

function receiveLeaderboard(leaderboard: Leaderboard): Action {
  return { type: 'RECEIVE_LEADERBOARD', leaderboard }
}
