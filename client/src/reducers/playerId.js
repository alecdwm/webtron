import { RECEIVE_PLAYER_ID } from '/actions'
import createReducer from '/utils/createReducer'

const initialState = null

export default createReducer(initialState, {
  [RECEIVE_PLAYER_ID]: (store, action) => action.playerId,
})
