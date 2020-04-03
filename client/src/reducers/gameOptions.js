import { SET_GAME_OPTIONS } from 'actions'
import createReducer from 'utils/createReducer'

const initialState = {
  debugReducers: true, //global.devMode,
}

export default createReducer(initialState, {
  [SET_GAME_OPTIONS]: (store, action) => ({
    ...store,
    ...action.gameOptions,
  }),
})
