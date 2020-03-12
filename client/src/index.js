import '/index.css'

import Root from '/components/Root'
import React from 'react'
import { render } from 'react-dom'

const webtron = document.getElementById('webtron')
webtron.classList.add('loaded')

render(<Root />, webtron)
