import { useReducer } from 'react'

export default function useForceUpdate() {
  const [, forceUpdate] = useReducer((x) => (x + 1) % 16384, 0)
  return forceUpdate
}
