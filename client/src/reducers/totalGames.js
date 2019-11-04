import { RECEIVE_TOTAL_GAMES } from 'actions'
import createReducer from 'utils/createReducer'

const initialState = 0

export default createReducer(initialState, {
  [RECEIVE_TOTAL_GAMES]: (store, action) => action.totalGames,
})
