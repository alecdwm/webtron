import type { Action, Dispatch, Store } from '@/types'

export type StoreApi = {
  getState: () => Store
  dispatch: Dispatch
  subscribe: (listener: () => void) => () => void
}

// A store in the shape useSyncExternalStore reads. dispatch runs the reducer
// straight away, so getState() inside a thunk always sees the actions it has
// already dispatched. Dispatching a function calls it with dispatch and
// getState.
export default function createStore(reducer: (state: Store | undefined, action: Action) => Store): StoreApi {
  let state = reducer(undefined, { type: 'INIT_STORE' })
  const listeners = new Set<() => void>()

  const getState = () => state
  const dispatch: Dispatch = (action) => {
    if (typeof action === 'function') return action(dispatch, getState)
    if (import.meta.env.DEV) console.log(action)
    const nextState = reducer(state, action)
    if (nextState === state) return
    state = nextState
    listeners.forEach((listener) => listener())
  }
  const subscribe = (listener: () => void) => {
    listeners.add(listener)
    return () => {
      listeners.delete(listener)
    }
  }

  return { getState, dispatch, subscribe }
}
