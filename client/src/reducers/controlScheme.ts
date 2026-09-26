import type { ControlScheme } from '@/types'
import { readControlScheme } from '@/utils/controlScheme'
import createReducer from '@/utils/createReducer'

export default createReducer<ControlScheme>(readControlScheme(), {
  SET_CONTROL_SCHEME: (_, { controlScheme }) => controlScheme,
})
