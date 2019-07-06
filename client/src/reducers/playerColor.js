import { SET_PLAYER_COLOR } from 'actions'
import createReducer from 'utils/createReducer'
import { randomColor } from 'utils/colors'

const initialState = randomColor()

export default createReducer(initialState, {
  [SET_PLAYER_COLOR]: (store, action) => action.color,
})
