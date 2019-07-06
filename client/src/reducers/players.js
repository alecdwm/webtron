import { RECEIVE_LOBBY_DATA } from 'actions'
import createReducer from 'utils/createReducer'

const initialState = { byId: {}, allIds: [] }

export default createReducer(initialState, {
  [RECEIVE_LOBBY_DATA]: (store, action) => {
    const byId = {}
    const allIds = []
    for (const player of action.players) {
      byId[player.id] = player
      allIds.push(player.id)
    }

    return { byId, allIds }
  },
})
