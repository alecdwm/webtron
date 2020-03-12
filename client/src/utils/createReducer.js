export default function createReducer(initialState, handlers) {
  return function reducer(state = initialState, action) {
    if (Object.prototype.hasOwnProperty.call(handlers, action.type)) {
      return handlers[action.type](state, action)
    } else if (Object.prototype.hasOwnProperty.call(handlers, 'default')) {
      return handlers['default'](state, action)
    } else {
      return state
    }
  }
}
