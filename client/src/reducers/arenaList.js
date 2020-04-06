import { RECEIVE_ARENA_LIST } from 'actions'
import createReducer from 'utils/createReducer'

const initialState = []

export default createReducer(initialState, {
  [RECEIVE_ARENA_LIST]: (_, action) => action.arenaList,
})
