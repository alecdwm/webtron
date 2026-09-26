import type { Action } from '@/types'

type Handlers<S> = { [T in Action['type']]?: (state: S, action: Extract<Action, { type: T }>) => S }

// Builds a reducer from one handler per action type. Actions without a
// handler leave the state as it is.
export default function createReducer<S>(initialState: S, handlers: Handlers<S>) {
  return function reducer(state: S = initialState, action: Action): S {
    const handler = handlers[action.type] as ((state: S, action: Action) => S) | undefined
    return handler ? handler(state, action) : state
  }
}
