import 'index.css'

import Root from 'components/Root'
import WebtronLogo from 'img/webtron.svg'
import React, { useEffect } from 'react'

export default function IndexPage() {
  useEffect(() => {
    const webtron = document.getElementById('webtron')
    webtron.classList.add('loaded')
  }, [])

  return (
    <>
      <header>
        <img src={WebtronLogo} alt="Webtron" />
      </header>
      <main id="webtron">
        <Root />
      </main>
      <footer>
        <p>
          <a href="https://owls.io">owls.io</a>
        </p>
      </footer>
    </>
  )
}
