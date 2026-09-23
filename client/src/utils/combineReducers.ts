type Reducers<S> = { [K in keyof S]: (state: S[K] | undefined, action) => S[K] }

export default function combineReducers<S extends object>(reducers: Reducers<S>) {
  return function combinedReducer(store: S | undefined, action): S {
    const config = store && 'config' in store ? (store.config as { debugReducers?: boolean }) : undefined
    if (config && config.debugReducers) console.log(action)
    const nextStore = {} as S
    let changed = false
    for (const key of Object.keys(reducers) as (keyof S)[]) {
      const prevState = store ? store[key] : undefined
      const nextState = reducers[key](prevState, action)
      if (nextState === undefined) {
        throw new Error(
          `Given ${(action && action.type) || 'an action'}, reducer '${String(key)}' returned undefined.\n` +
            `To ignore an action, you must explicitly return the previous state.\n` +
            `If you want this reducer to hold no value, you can return null instead of undefined.`,
        )
      }
      nextStore[key] = nextState
      changed = changed || prevState !== nextState
    }
    return changed || store === undefined ? nextStore : store
  }
}
