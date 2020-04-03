import { RECEIVE_GAME_PLAYERS } from 'actions'
import createReducer from 'utils/createReducer'

const initialState = { byId: {}, allIds: [] }

export default createReducer(initialState, {
  [RECEIVE_GAME_PLAYERS]: (store, action) => {
    const byId = {}
    const allIds = []
    for (const player of action.gamePlayers) {
      byId[player.id] = player
      allIds.push(player.id)
    }

    return { byId, allIds }
  },
})
