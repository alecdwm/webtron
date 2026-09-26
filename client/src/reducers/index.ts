import type { Store } from '@/types'
import combineReducers from '@/utils/combineReducers'

import arena from './arena'
import arenaList from './arenaList'
import config from './config'
import joinError from './joinError'
import leaderboard from './leaderboard'
import player from './player'
import preloadedImages from './preloadedImages'
import socketState from './socketState'
import stage from './stage'

export default combineReducers<Store>({
  arena,
  arenaList,
  config,
  joinError,
  leaderboard,
  player,
  preloadedImages,
  socketState,
  stage,
})
