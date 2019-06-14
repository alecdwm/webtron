import React from 'react'
import { hot } from 'react-hot-loader/root'
import usePreventWebtronContextMenu from 'hooks/usePreventWebtronContextMenu'
import useThunkReducer from 'hooks/useThunkReducer'
import { StoreDispatch } from 'hooks/useStoreDispatch'
import reducers from 'reducers'
import Webtron from 'components/Webtron'

const initialStore = reducers(undefined, { type: 'INIT_STORE' })

export default hot(function Main() {
  const [store, dispatch] = useThunkReducer(reducers, initialStore)
  usePreventWebtronContextMenu()

  return (
    <StoreDispatch.Provider value={dispatch}>
      <Webtron store={store} />
    </StoreDispatch.Provider>
  )
})
