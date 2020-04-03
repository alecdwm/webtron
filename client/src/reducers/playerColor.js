import { SET_PLAYER_COLOR } from 'actions'
import { randomColor } from 'utils/colors'
import createReducer from 'utils/createReducer'

const initialState = randomColor()

export default createReducer(initialState, {
  [SET_PLAYER_COLOR]: (store, action) => action.color,
})
