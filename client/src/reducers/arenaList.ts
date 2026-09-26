import type { ArenaOverview } from '@/types'
import createReducer from '@/utils/createReducer'

export default createReducer<ArenaOverview[] | null>(null, {
  RECEIVE_ARENA_LIST: (_, { arenaList }) => arenaList,
  SET_SOCKET_STATE: (arenaList, { socketState }) => (socketState === 'OPEN' ? null : arenaList),
})
