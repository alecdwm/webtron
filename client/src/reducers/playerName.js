import { SET_PLAYER_NAME } from 'actions'
import createReducer from 'utils/createReducer'

const initialState = ''

export default createReducer(initialState, {
  [SET_PLAYER_NAME]: (store, action) => action.name,
})
