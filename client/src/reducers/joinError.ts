import { RECEIVE_NAME_IN_USE, SET_PLAYER_NAME, SET_SOCKET_STATE } from '@/actions'
import createReducer from '@/utils/createReducer'
import socketStates from '@/utils/socketStates'

const initialState = null

// Why the server last refused the player, until they change their name or
// connect again.
export default createReducer(initialState, {
  [RECEIVE_NAME_IN_USE]: () => 'NAME ALREADY IN USE',
  [SET_PLAYER_NAME]: () => null,
  [SET_SOCKET_STATE]: (joinError, { socketState }) => (socketState === socketStates.CONNECTING ? null : joinError),
})
