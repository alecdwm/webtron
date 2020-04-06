import { SocketStates } from 'hooks/useSocket'
import createSimpleAction from 'utils/createSimpleAction'

export const SET_CONFIG = 'SET_CONFIG'
export const SET_STAGE = 'SET_STAGE'
export const PRELOAD_IMAGES = 'PRELOAD_IMAGES'

export const setConfig = createSimpleAction(SET_CONFIG, 'config')
export const setStage = createSimpleAction(SET_STAGE, 'stage')
export const preloadImages = createSimpleAction(PRELOAD_IMAGES, 'urls')

export const SET_PLAYER_NAME = 'SET_PLAYER_NAME'
export const SET_PLAYER_COLOR = 'SET_PLAYER_COLOR'

export const setPlayerName = createSimpleAction(SET_PLAYER_NAME, 'name')
export const setPlayerColor = createSimpleAction(SET_PLAYER_COLOR, 'color')

export const SET_SOCKET_STATE = 'SET_SOCKET_STATE'
export const RECEIVE_SOCKET_MESSAGE = 'RECEIVE_SOCKET_MESSAGE'

export function setSocketState(socketState) {
  return (dispatch) => {
    if (socketState === SocketStates.CONNECTING) dispatch(setStage('Connect'))
    if (socketState === SocketStates.OPEN) dispatch(setStage('ArenaSelect'))
    if (socketState === SocketStates.CLOSED) dispatch(setStage('MainMenu'))
    dispatch({ type: SET_SOCKET_STATE, socketState })
  }
}
export function receiveSocketMessage(messageType, messageData) {
  return (dispatch) => {
    const typeHandlers = {
      ArenaList: receiveArenaList,
      ArenaJoined: receiveArenaJoined,
      ArenaState: receiveArenaState,
      ArenaStatePatch: receiveArenaStatePatch,
    }

    if (typeHandlers[messageType]) return dispatch(typeHandlers[messageType](messageData))
    console.warn('Unhandled message received', messageType, messageData)
  }
}

export const RECEIVE_ARENA_LIST = 'RECEIVE_ARENA_LIST'
export const RECEIVE_ARENA_JOINED = 'RECEIVE_ARENA_JOINED'
export const RECEIVE_ARENA_STATE = 'RECEIVE_ARENA_STATE'
export const RECEIVE_ARENA_STATE_PATCH = 'RECEIVE_ARENA_STATE_PATCH'

export const receiveArenaList = createSimpleAction(RECEIVE_ARENA_LIST, 'arenaList')
export const receiveArenaJoined = createSimpleAction(RECEIVE_ARENA_JOINED, 'arenaId')
export const receiveArenaState = createSimpleAction(RECEIVE_ARENA_STATE, 'state')
export const receiveArenaStatePatch = createSimpleAction(RECEIVE_ARENA_STATE_PATCH, 'statePatch')
