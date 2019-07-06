import { SocketStates } from 'hooks/useSocket'
import createSimpleAction from 'utils/createSimpleAction'

export const SET_GAME_OPTIONS = 'SET_GAME_OPTIONS'
export const SET_GAME_STATE = 'SET_GAME_STATE'
export const PRELOAD_IMAGES = 'PRELOAD_IMAGES'

export const setGameOptions = createSimpleAction(SET_GAME_OPTIONS, 'gameOptions')
export const setGameState = createSimpleAction(SET_GAME_STATE, 'state')
export const preloadImages = createSimpleAction(PRELOAD_IMAGES, 'urls')

export const SET_PLAYER_NAME = 'SET_PLAYER_NAME'
export const SET_PLAYER_COLOR = 'SET_PLAYER_COLOR'

export const setPlayerName = createSimpleAction(SET_PLAYER_NAME, 'name')
export const setPlayerColor = createSimpleAction(SET_PLAYER_COLOR, 'color')

export const SET_STATUS_TEXT = 'SET_STATUS_TEXT'
export const SET_SOCKET_STATE = 'SET_SOCKET_STATE'
export const RECEIVE_SOCKET_MESSAGE = 'RECEIVE_SOCKET_MESSAGE'
export const RECEIVE_PLAYER_ID = 'RECEIVE_PLAYER_ID'
export const RECEIVE_LOBBY_DATA = 'RECEIVE_LOBBY_DATA'

export const setStatusText = createSimpleAction(SET_STATUS_TEXT, 'text')
export function setSocketState(socketState) {
  return (dispatch, getState) => {
    if (socketState === SocketStates.CONNECTING) {
      if (getState().playerName === '') dispatch(setPlayerName('anon'))
      dispatch(setGameState('Connect'))
    }
    dispatch({ type: SET_SOCKET_STATE, socketState })
  }
}
export function receiveSocketMessage(messageType, messageData) {
  return dispatch => {
    switch (messageType) {
      case 'PlayerId':
        return dispatch(receivePlayerId(messageData))

      case 'LobbyData':
        return dispatch(receiveLobbyData(messageData.games, messageData.players))

      default:
        console.warn('Unhandled message received', messageType, messageData)
    }
  }
}
export const receivePlayerId = createSimpleAction(RECEIVE_PLAYER_ID, 'playerId')
export const receiveLobbyData = createSimpleAction(RECEIVE_LOBBY_DATA, 'games', 'players')
