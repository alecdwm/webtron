import type { SocketState } from '@/types'
import createReducer from '@/utils/createReducer'

export default createReducer<SocketState>('NOT_CONNECTED', {
  SET_SOCKET_STATE: (_, { socketState }) => socketState,
})
