import React from 'react'
import { render } from 'react-dom'
import Root from 'components/Root'
import 'index.css'

window.addEventListener('load', () => {
  const webtron = document.getElementById('webtron')
  webtron.classList.add('loaded')

  render(<Root />, webtron)
})
