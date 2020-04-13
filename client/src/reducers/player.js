import { RECEIVE_ARENA_JOINED, SET_PLAYER_COLOR, SET_PLAYER_NAME } from 'actions'
import { randomColor } from 'utils/colors'
import createReducer from 'utils/createReducer'

const initialState = { id: null, name: '', color: randomColor() }

export default createReducer(initialState, {
  [RECEIVE_ARENA_JOINED]: (store, action) => ({ ...store, id: action.playerId }),
  [SET_PLAYER_NAME]: (store, action) => ({ ...store, name: action.name }),
  [SET_PLAYER_COLOR]: (store, action) => ({ ...store, color: action.color }),
})
