export default function combineReducers(reducers) {
  return function combinedReducer(store = {}, action) {
    if (store && store.config && store.config.debugReducers) console.log(action)
    const nextStore = {}
    let changed = false
    for (const [key, reducer] of Object.entries(reducers)) {
      const prevState = store[key]
      const nextState = reducer(prevState, action)
      if (nextState === undefined) {
        throw new Error(
          console.error(
            `Given ${(action && action.type) || 'an action'}, reducer '${key}' returned undefined.\n` +
              `To ignore an action, you must explicitly return the previous state.\n` +
              `If you want this reducer to hold no value, you can return null instead of undefined.`,
          ),
        )
      }
      nextStore[key] = nextState
      changed = changed || prevState !== nextState
    }
    return changed ? nextStore : store
  }
}
