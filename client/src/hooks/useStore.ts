import { createContext, useContext } from 'react'

import type { Store } from '@/types'

export const StoreContext = createContext<Store | null>(null)

export default function useStore(): Store {
  const store = useContext(StoreContext)
  if (store === null) throw new Error('useStore must be used inside a StoreContext provider')
  return store
}
