import { SET_SOCKET_STATE } from 'actions'
import { SocketStates } from 'hooks/useSocket'
import createReducer from 'utils/createReducer'

const initialState = SocketStates.NOT_CONNECTED

export default createReducer(initialState, {
  [SET_SOCKET_STATE]: (store, action) => action.socketState,
})
