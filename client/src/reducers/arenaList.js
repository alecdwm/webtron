import { RECEIVE_ARENA_LIST, SET_SOCKET_STATE } from 'actions'
import createReducer from 'utils/createReducer'
import socketStates from 'utils/socketStates'

const initialState = null

export default createReducer(initialState, {
  [RECEIVE_ARENA_LIST]: (_, action) => action.arenaList,
  [SET_SOCKET_STATE]: (store, { socketState }) => (socketState === socketStates.OPEN ? null : store),
})
