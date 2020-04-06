import { SET_PLAYER_COLOR, SET_PLAYER_NAME } from 'actions'
import { randomColor } from 'utils/colors'
import createReducer from 'utils/createReducer'

const initialState = { name: '', color: randomColor() }

export default createReducer(initialState, {
  [SET_PLAYER_NAME]: (store, action) => ({ ...store, name: action.name }),
  [SET_PLAYER_COLOR]: (store, action) => ({ ...store, color: action.color }),
})
