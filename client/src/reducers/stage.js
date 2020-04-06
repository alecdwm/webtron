import { SET_STAGE } from 'actions'
import createReducer from 'utils/createReducer'

const initialState = 'MainMenu'

export default createReducer(initialState, {
  [SET_STAGE]: (_, action) => action.stage,
})
