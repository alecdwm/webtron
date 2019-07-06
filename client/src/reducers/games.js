import { RECEIVE_LOBBY_DATA } from 'actions'
import createReducer from 'utils/createReducer'

const initialState = { byId: {}, allIds: [] }

export default createReducer(initialState, {
  [RECEIVE_LOBBY_DATA]: (store, action) => {
    const byId = {}
    const allIds = []
    for (const game of action.games) {
      byId[game.id] = game
      allIds.push(game.id)
    }

    return { byId, allIds }
  },
})
