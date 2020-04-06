import combineReducers from 'utils/combineReducers'

import arenaList from './arenaList'
import config from './config'
import player from './player'
import preloadedImages from './preloadedImages'
import socketState from './socketState'
import stage from './stage'

export default combineReducers({
  config,
  player,
  preloadedImages,
  arenaList,
  socketState,
  stage,
})
