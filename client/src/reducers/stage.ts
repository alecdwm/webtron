import type { Stage } from '@/types'
import createReducer from '@/utils/createReducer'

export default createReducer<Stage>('MainMenu', {
  SET_STAGE: (_, { stage }) => stage,
})
