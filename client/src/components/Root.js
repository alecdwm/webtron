import Webtron from 'components/Webtron'
import usePreventWebtronContextMenu from 'hooks/usePreventWebtronContextMenu'
import { StoreContext } from 'hooks/useStore'
import { StoreDispatchContext } from 'hooks/useStoreDispatch'
import useThunkReducer from 'hooks/useThunkReducer'
import React from 'react'
import reducers from 'reducers'

const initialStore = reducers(undefined, { type: 'INIT_STORE' })

export default function Root() {
  const [store, dispatch] = useThunkReducer(reducers, initialStore)
  usePreventWebtronContextMenu()

  return (
    <StoreContext.Provider value={store}>
      <StoreDispatchContext.Provider value={dispatch}>
        <Webtron />
      </StoreDispatchContext.Provider>
    </StoreContext.Provider>
  )
}
