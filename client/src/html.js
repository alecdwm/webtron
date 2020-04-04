import gridbikeBlue from 'img/gridbike-blue.png'
import PropTypes from 'prop-types'
import React from 'react'

export default function HTML(props) {
  return (
    <html lang="en" {...props.htmlAttributes}>
      <head>
        <meta charSet="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <meta
          name="description"
          content="A networked-multiplayer clone of the classic 'Tron Lightcycles' arcade game!"
        />
        <title>Webtron</title>
        <link rel="icon" type="image/png" href={gridbikeBlue} />
        {props.headComponents}
      </head>
      <body {...props.bodyAttributes}>
        {props.preBodyComponents}
        <div key={`body`} id="___gatsby" dangerouslySetInnerHTML={{ __html: props.body }} />
        {props.postBodyComponents}
      </body>
    </html>
  )
}

HTML.propTypes = {
  htmlAttributes: PropTypes.object,
  headComponents: PropTypes.array,
  bodyAttributes: PropTypes.object,
  preBodyComponents: PropTypes.array,
  body: PropTypes.string,
  postBodyComponents: PropTypes.array,
}
