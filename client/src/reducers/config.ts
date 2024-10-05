import { SET_CONFIG } from '@/actions'
import createReducer from '@/utils/createReducer'

const initialState = {
  debugReducers: import.meta.env.MODE === 'development',
}

export default createReducer(initialState, {
  [SET_CONFIG]: (store, action) => ({
    ...store,
    ...action.config,
  }),
})
