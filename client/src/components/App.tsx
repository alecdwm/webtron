import '@/index.css'

import Webtron from '@/components/Webtron'
import useAddWebtronLoadedClass from '@/hooks/useAddWebtronLoadedClass'
import usePreventWebtronContextMenu from '@/hooks/usePreventWebtronContextMenu'
import { StoreContext } from '@/hooks/useStore'
import { StoreDispatchContext } from '@/hooks/useStoreDispatch'
import useThunkReducer from '@/hooks/useThunkReducer'
import reducers from '@/reducers'

const initialStore = reducers(undefined, { type: 'INIT_STORE' })

export default function App() {
  usePreventWebtronContextMenu()
  useAddWebtronLoadedClass()

  const [store, dispatch] = useThunkReducer(reducers, initialStore)

  return (
    <StoreContext.Provider value={store}>
      <StoreDispatchContext.Provider value={dispatch}>
        <Webtron />
      </StoreDispatchContext.Provider>
    </StoreContext.Provider>
  )
}
