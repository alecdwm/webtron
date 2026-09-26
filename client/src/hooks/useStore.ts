import { createContext, useContext, useSyncExternalStore } from 'react'

import type { StoreApi } from '@/store/createStore'
import type { Store } from '@/types'

export const StoreContext = createContext<StoreApi | null>(null)

export function useStoreApi(): StoreApi {
  const store = useContext(StoreContext)
  if (store === null) throw new Error('useStore must be used inside a StoreContext provider')
  return store
}

// Reads part of the store, re-rendering only when that part changes. The
// selector must return a value already in the store, not build a new one.
export default function useStore<T>(selector: (store: Store) => T): T {
  const { getState, subscribe } = useStoreApi()
  return useSyncExternalStore(subscribe, () => selector(getState()))
}
