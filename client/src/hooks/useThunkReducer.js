import { useCallback, useReducer, useRef } from 'react'

export default function useThunkReducer(reducer, initialState) {
  const [state, dispatch] = useReducer(reducer, initialState)

  const stateRef = useRef(state)
  stateRef.current = state

  const getState = useCallback(() => stateRef.current, [])
  const thunkMiddleware = useCallback(
    (dispatch) => (actionCreator) => {
      if (typeof actionCreator !== 'function') {
        return dispatch(actionCreator)
      }
      return actionCreator(dispatch, getState)
    },
    [getState],
  )

  const thunkDispatch = useCallback((actionCreator) => thunkMiddleware(dispatch)(actionCreator), [
    thunkMiddleware,
    dispatch,
  ])

  return [state, thunkDispatch]
}
