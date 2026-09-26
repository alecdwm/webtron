import type { Store } from '@/types'
import combineReducers from '@/utils/combineReducers'

import arena from './arena'
import arenaList from './arenaList'
import joinError from './joinError'
import leaderboard from './leaderboard'
import player from './player'
import serverTimeOffset from './serverTimeOffset'
import socketState from './socketState'
import stage from './stage'

export default combineReducers<Store>({
  arena,
  arenaList,
  joinError,
  leaderboard,
  player,
  serverTimeOffset,
  socketState,
  stage,
})
