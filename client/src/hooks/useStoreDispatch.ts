import { createContext, useContext } from 'react'

export const StoreDispatchContext = createContext(null)

export default function useStoreDispatch() {
  const dispatch = useContext(StoreDispatchContext)
  return dispatch
}
