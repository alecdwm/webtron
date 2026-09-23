import { useCallback, useLayoutEffect, useReducer, useRef } from 'react'

export default function useThunkReducer(reducer, initialState) {
  const [state, dispatch] = useReducer(reducer, initialState)

  const stateRef = useRef(state)
  useLayoutEffect(() => {
    stateRef.current = state
  }, [state])

  const getState = useCallback(() => stateRef.current, [])
  const thunkDispatch = useCallback(
    (action) => {
      const run = (action) => (typeof action === 'function' ? action(run, getState) : dispatch(action))
      return run(action)
    },
    [getState, dispatch],
  )

  return [state, thunkDispatch]
}
