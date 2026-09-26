import createReducer from '@/utils/createReducer'

export default createReducer<number>(0, {
  RECEIVE_SERVER_TIME: (_, { serverTimeOffset }) => serverTimeOffset,
})
