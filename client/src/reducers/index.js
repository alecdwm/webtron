import combineReducers from 'utils/combineReducers'

import config from './config'
import gameState from './gameState'
import player from './player'
import preloadedImages from './preloadedImages'
import socketState from './socketState'

export default combineReducers({
  config,
  gameState,
  player,
  preloadedImages,
  socketState,
})
