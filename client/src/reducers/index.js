import combineReducers from 'utils/combineReducers'

import gameOptions from './gameOptions'
import gameState from './gameState'
import playerColor from './playerColor'
import playerId from './playerId'
import playerName from './playerName'
import players from './players'
import preloadedImages from './preloadedImages'
import socketState from './socketState'
import statusText from './statusText'

export default combineReducers({
  gameOptions,
  gameState,
  preloadedImages,

  playerName,
  playerColor,

  statusText,
  socketState,
  playerId,
  players,
})
