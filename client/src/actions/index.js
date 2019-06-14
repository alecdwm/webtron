import { SocketStates } from 'hooks/useSocket'

export const SET_GAME_STATE = 'SET_GAME_STATE'
export const PRELOAD_IMAGES = 'PRELOAD_IMAGES'

export const SET_PLAYER_NAME = 'SET_PLAYER_NAME'
export const SET_PLAYER_COLOR = 'SET_PLAYER_COLOR'

export const SET_STATUS_TEXT = 'SET_STATUS_TEXT'
export const SET_SOCKET_STATE = 'SET_SOCKET_STATE'
export const RECEIVE_SOCKET_MESSAGE = 'RECEIVE_SOCKET_MESSAGE'
export const RECEIVE_GAMES_LIST = 'RECEIVE_GAMES_LIST'

export function setGameState(state) {
  return {
    type: SET_GAME_STATE,
    state,
  }
}
export function preloadImages(urls) {
  return {
    type: PRELOAD_IMAGES,
    urls: urls,
  }
}

export function setPlayerName(name) {
  return {
    type: SET_PLAYER_NAME,
    name,
  }
}
export function setPlayerColor(color) {
  return {
    type: SET_PLAYER_COLOR,
    color,
  }
}

export function setStatusText(text) {
  return {
    type: SET_STATUS_TEXT,
    text,
  }
}
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
      case 'GamesList':
        return dispatch(receiveGamesList(messageData.games))

      default:
        console.warn('Unhandled message received', messageType, messageData)
    }
  }
}
export function receiveGamesList(games) {
  return {
    type: RECEIVE_GAMES_LIST,
    games,
  }
}
