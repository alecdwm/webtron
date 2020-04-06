import combineReducers from 'utils/combineReducers'

import config from './config'
import player from './player'
import preloadedImages from './preloadedImages'
import socketState from './socketState'
import stage from './stage'

export default combineReducers({
  config,
  player,
  preloadedImages,
  socketState,
  stage,
})
