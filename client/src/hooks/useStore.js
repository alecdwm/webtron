import { createContext, useContext } from 'react'

export const StoreContext = createContext(null)

export default function useStore() {
  const store = useContext(StoreContext)
  return store
}
