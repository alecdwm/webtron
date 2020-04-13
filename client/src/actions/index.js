import { getArenaList, join } from 'actions/socket'
import createSimpleAction from 'utils/createSimpleAction'
import socketStates from 'utils/socketStates'
export * from 'actions/socket'

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
    dispatch({ type: SET_SOCKET_STATE, socketState })
    if (socketState === socketStates.CONNECTING) dispatch(setStage('Connect'))
    if (socketState === socketStates.OPEN) dispatch(getArenaList())
    if (socketState === socketStates.CLOSED) dispatch(setStage('MainMenu'))
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

export function receiveArenaList(arenaList) {
  return (dispatch, getState) => {
    const prevArenaList = getState().arenaList
    dispatch({ type: RECEIVE_ARENA_LIST, arenaList })
    if (prevArenaList === null) {
      if (arenaList.length < 1) dispatch(join())
      else dispatch(setStage('ArenaSelect'))
    }
  }
}
export function receiveArenaJoined([arenaId, playerId]) {
  return (dispatch) => {
    dispatch({ type: RECEIVE_ARENA_JOINED, arenaId, playerId })
    dispatch(setStage('Arena'))
  }
}
export const receiveArenaState = createSimpleAction(RECEIVE_ARENA_STATE, 'state')
export const receiveArenaStatePatch = createSimpleAction(RECEIVE_ARENA_STATE_PATCH, 'statePatch')
