import { SET_STATUS_TEXT } from '/actions'
import createReducer from '/utils/createReducer'

const initialState = ''

export default createReducer(initialState, {
  [SET_STATUS_TEXT]: (store, action) => action.text,
})
