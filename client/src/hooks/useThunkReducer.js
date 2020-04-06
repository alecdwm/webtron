import { useCallback, useReducer, useRef } from 'react'

export default function useThunkReducer(reducer, initialState) {
  const [state, dispatch] = useReducer(reducer, initialState)

  const stateRef = useRef(state)
  stateRef.current = state

  const getState = useCallback(() => stateRef.current, [])
  const thunkDispatch = useCallback(
    (action) => (typeof action === 'function' ? action(thunkDispatch, getState) : dispatch(action)),
    [getState, dispatch],
  )

  return [state, thunkDispatch]
}
