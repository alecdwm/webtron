import type { Store } from '@/types'
import combineReducers from '@/utils/combineReducers'

import arena from './arena'
import arenaList from './arenaList'
import config from './config'
import leaderboard from './leaderboard'
import player from './player'
import preloadedImages from './preloadedImages'
import socketState from './socketState'
import stage from './stage'

export default combineReducers<Store>({
  arena,
  arenaList,
  config,
  leaderboard,
  player,
  preloadedImages,
  socketState,
  stage,
})
