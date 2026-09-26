import createReducer from '@/utils/createReducer'

// Why the server last refused the player, until they change their name or
// connect again.
export default createReducer<string | null>(null, {
  RECEIVE_NAME_IN_USE: () => 'NAME ALREADY IN USE',
  SET_PLAYER_NAME: () => null,
  SET_SOCKET_STATE: (joinError, { socketState }) => (socketState === 'CONNECTING' ? null : joinError),
})
