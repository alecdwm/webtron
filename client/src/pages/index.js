import 'index.css'

import WebtronLogo from 'img/webtron.svg'
import React, { Suspense, useEffect } from 'react'

const ClientSideOnlyRoot = React.lazy(() => import('components/Root'))

export default function IndexPage() {
  useEffect(() => {
    const webtron = document.getElementById('webtron')
    webtron.classList.add('loaded')
  }, [])

  const isSSR = typeof window === 'undefined'
  if (isSSR) return null

  return (
    <>
      <header>
        <img src={WebtronLogo} alt="Webtron" />
      </header>
      <main id="webtron">
        <Suspense fallback={<div />}>
          <ClientSideOnlyRoot />
        </Suspense>
      </main>
      <footer>
        <p>
          <a href="https://owls.io">owls.io</a>
        </p>
      </footer>
    </>
  )
}
