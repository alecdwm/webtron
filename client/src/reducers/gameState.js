import { SET_GAME_STATE } from '/actions'
import createReducer from '/utils/createReducer'

const initialState = 'MainMenu'

export default createReducer(initialState, {
  [SET_GAME_STATE]: (store, action) => action.state,
})
