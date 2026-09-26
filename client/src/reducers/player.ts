import type { PlayerSettings } from '@/types'
import { randomColor } from '@/utils/colors'
import createReducer from '@/utils/createReducer'

export default createReducer<PlayerSettings>(
  { id: null, name: '', color: randomColor() },
  {
    RECEIVE_ARENA_JOINED: (player, { playerId }) => ({ ...player, id: playerId }),
    SET_PLAYER_NAME: (player, { name }) => ({ ...player, name }),
    SET_PLAYER_COLOR: (player, { color }) => ({ ...player, color }),
  },
)
