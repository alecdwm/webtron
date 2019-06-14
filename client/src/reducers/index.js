import {
  SET_GAME_STATE,
  PRELOAD_IMAGES,
  SET_PLAYER_NAME,
  SET_PLAYER_COLOR,
  SET_STATUS_TEXT,
  SET_SOCKET_STATE,
  RECEIVE_GAMES_LIST,
} from 'actions'
import { SocketStates } from 'hooks/useSocket'
import { randomColor } from 'utils/colors'

const INIT_STORE = 'INIT_STORE'

export const initialStore = {
  gameState: 'MainMenu',
  preloadedImages: [],
  playerName: '',
  playerColor: randomColor(),
  statusText: '',
  socketState: SocketStates.NOT_CONNECTED,
  gamesList: [],
}

export default function storeReducer(store = initialStore, action) {
  switch (action.type) {
    case SET_GAME_STATE:
      return { ...store, gameState: action.state }
    case PRELOAD_IMAGES:
      return {
        ...store,
        preloadedImages: action.urls.map(url => {
          const image = new Image()
          image.src = url
          return image
        }),
      }

    case SET_PLAYER_NAME:
      return { ...store, playerName: action.name }
    case SET_PLAYER_COLOR:
      return { ...store, playerColor: action.color }

    case SET_STATUS_TEXT:
      return { ...store, statusText: action.text }
    case SET_SOCKET_STATE:
      return { ...store, socketState: action.socketState }
    case RECEIVE_GAMES_LIST:
      return { ...store, gamesList: action.games }

    case INIT_STORE:
      return store

    default:
      console.warn('Action not handled! ', action)
  }
  return store
}
