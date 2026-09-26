import type { PlayerSettings } from '@/types'
import createReducer from '@/utils/createReducer'
import { readPlayerColor, readPlayerName } from '@/utils/playerSettings'

export default createReducer<PlayerSettings>(
  { id: null, name: readPlayerName(), color: readPlayerColor() },
  {
    RECEIVE_ARENA_JOINED: (player, { playerId }) => ({ ...player, id: playerId }),
    SET_PLAYER_NAME: (player, { name }) => ({ ...player, name }),
    SET_PLAYER_COLOR: (player, { color }) => ({ ...player, color }),
  },
)
