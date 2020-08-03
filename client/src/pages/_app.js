/* global require, module */

import 'index.css'

import { StoreContext } from 'hooks/useStore'
import { StoreDispatchContext } from 'hooks/useStoreDispatch'
import useThunkReducer from 'hooks/useThunkReducer'
import PropTypes from 'prop-types'
import React from 'react'

const isSSR = typeof window === 'undefined'

let reducers = require('reducers').default
const initialStore = reducers(undefined, { type: 'INIT_STORE' })

if (!isSSR && module.hot)
  module.hot.accept('reducers', () => {
    reducers = require('reducers').default
  })

export default function App({ Component, pageProps }) {
  const [store, dispatch] = useThunkReducer(reducers, initialStore)

  return (
    <StoreContext.Provider value={store}>
      <StoreDispatchContext.Provider value={dispatch}>
        <Component {...pageProps} />
      </StoreDispatchContext.Provider>
    </StoreContext.Provider>
  )
}
App.propTypes = {
  Component: PropTypes.func,
  pageProps: PropTypes.object,
}