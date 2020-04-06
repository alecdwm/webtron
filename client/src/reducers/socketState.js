import { SET_SOCKET_STATE } from 'actions'
import createReducer from 'utils/createReducer'
import socketStates from 'utils/socketStates'

const initialState = socketStates.NOT_CONNECTED

export default createReducer(initialState, {
  [SET_SOCKET_STATE]: (_, action) => action.socketState,
})
