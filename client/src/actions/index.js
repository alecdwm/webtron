import { SocketStates } from '/hooks/useSocket'
import createSimpleAction from '/utils/createSimpleAction'

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
export const RECEIVE_TOTAL_GAMES = 'RECEIVE_TOTAL_GAMES'
export const RECEIVE_JOINED_GAME = 'RECEIVE_JOINED_GAME'
export const RECEIVE_PARTED_GAME = 'RECEIVE_PARTED_GAME'
export const RECEIVE_GAME_PLAYERS = 'RECEIVE_GAME_PLAYERS'
export const RECEIVE_GAME_STARTING = 'RECEIVE_GAME_STARTING'
export const RECEIVE_NEW_GAME_STATE = 'RECEIVE_NEW_GAME_STATE'
export const RECEIVE_PATCH_GAME_STATE = 'RECEIVE_PATCH_GAME_STATE'

export const setStatusText = createSimpleAction(SET_STATUS_TEXT, 'text')
export function setSocketState(socketState) {
  return (dispatch, getState) => {
    // if (socketState === SocketStates.CONNECTING) {
    //   if (getState().playerName === '') dispatch(setPlayerName('anon'))
    //   dispatch(setGameState('Connect'))
    // }
    dispatch({ type: SET_SOCKET_STATE, socketState })
  }
}
export function receiveSocketMessage(messageType, messageData) {
  return dispatch => {
    switch (messageType) {
      case 'PlayerId':
        return dispatch(receivePlayerId(messageData))

      case 'TotalGames':
        return dispatch(receiveTotalGames(messageData))

      case 'JoinedGame':
        return dispatch(receiveJoinedGame(messageData))

      case 'PartedGame':
        return dispatch(receivePartedGame())

      case 'GamePlayers':
        return dispatch(receiveGamePlayers(messageData))

      case 'GameStarting':
        return dispatch(receiveGameStarting(messageData))

      case 'NewGameState':
        return dispatch(receiveNewGameState(messageData))

      case 'PatchGameState':
        return dispatch(receivePatchGameState(messageData))

      default:
        console.warn('Unhandled message received', messageType, messageData)
    }
  }
}
export const receivePlayerId = createSimpleAction(RECEIVE_PLAYER_ID, 'playerId')
export const receiveTotalGames = createSimpleAction(RECEIVE_TOTAL_GAMES, 'totalGames')
export const receiveJoinedGame = createSimpleAction(RECEIVE_JOINED_GAME, 'gameId')
export const receivePartedGame = createSimpleAction(RECEIVE_PARTED_GAME)
export const receiveGamePlayers = createSimpleAction(RECEIVE_GAME_PLAYERS, 'gamePlayers')
export const receiveGameStarting = createSimpleAction(RECEIVE_GAME_STARTING, 'gameStarting')
export const receiveNewGameState = createSimpleAction(RECEIVE_NEW_GAME_STATE, 'gameState')
export const receivePatchGameState = createSimpleAction(RECEIVE_PATCH_GAME_STATE, 'patchGameState')
