import combineReducers from 'utils/combineReducers'

import gameOptions from './gameOptions'
import gameState from './gameState'
import preloadedImages from './preloadedImages'
import playerName from './playerName'
import playerColor from './playerColor'
import statusText from './statusText'
import socketState from './socketState'
import playerId from './playerId'
import games from './games'
import players from './players'

export default combineReducers({
  gameOptions,
  gameState,
  preloadedImages,

  playerName,
  playerColor,

  statusText,
  socketState,
  playerId,
  games,
  players,
})
