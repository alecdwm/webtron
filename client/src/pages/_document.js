import lightcycleBlue from 'img/lightcycle-blue.png'
import webtron from 'img/webtron.svg'
import Document, { Head, Html, Main, NextScript } from 'next/document'
import React from 'react'

class MyDocument extends Document {
  render() {
    return (
      <Html lang="en">
        <Head>
          <title data-react-helmet="true">Webtron</title>
          <meta
            key="description"
            name="description"
            content="A browser-based multiplayer implementation of the classic Tron Lightcycles arcade game."
          />
          <link rel="shortcut icon" type="image/png" href={lightcycleBlue} />
        </Head>
        <body>
          <header>
            <img src={webtron} alt="Webtron" />
          </header>
          <main id="webtron">
            <Main />
          </main>
          <footer>
            <p>
              <a href="https://owls.io">owls.io</a>
            </p>
          </footer>
          <NextScript />
        </body>
      </Html>
    )
  }
}

export default MyDocument
