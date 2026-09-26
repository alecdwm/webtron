import { RECEIVE_ARENA_JOINED, RECEIVE_ARENA_STATE_PATCH, RECEIVE_LEADERBOARD } from '@/actions'
import createReducer from '@/utils/createReducer'

const initialState = null

// The leaderboard sent when the last round was won. It is cleared when a new
// round starts, so the next win never shows the previous round's standings.
export default createReducer(initialState, {
  [RECEIVE_LEADERBOARD]: (_, { leaderboard }) => leaderboard,
  [RECEIVE_ARENA_JOINED]: () => null,
  [RECEIVE_ARENA_STATE_PATCH]: (leaderboard, { statePatch = [] }) =>
    statePatch.some((update) => typeof update === 'object' && 'Start' in update) ? null : leaderboard,
})
