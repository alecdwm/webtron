import { useCallback, useState } from 'react'

export default function useBooleanState(initialState) {
  const [state, setState] = useState(Boolean(initialState))
  const setStateTrue = useCallback(() => setState(true), [setState])
  const setStateFalse = useCallback(() => setState(false), [setState])
  const toggleState = useCallback(() => setState(state => !state), [setState])
  return [state, setStateTrue, setStateFalse, toggleState]
}
