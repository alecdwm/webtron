import { createContext, useContext } from 'react'

export const StoreDispatch = createContext(null)

export default function useStoreDispatch() {
  const dispatch = useContext(StoreDispatch)
  return dispatch
}
