import useClassName from 'hooks/useClassName'
import useStore from 'hooks/useStore'
import PropTypes from 'prop-types'
import React from 'react'
import { colorToHexString } from 'utils/colors'

import styles from './Lightribbon.module.css'

export default function Lightribbon({ color, points }) {
  const {
    arena: { width, height },
  } = useStore()

  const polylinePoints = points.map((point) => `${point[0]}, ${width - point[1]}`).join(' ')

  const Lightribbon = useClassName(styles.lightribbon, 'svg')

  return (
    <Lightribbon viewBox={`0 0 ${width} ${height}`}>
      <polyline points={polylinePoints} stroke={colorToHexString(color)} strokeWidth="2" fill="none" />
    </Lightribbon>
  )
}
Lightribbon.propTypes = {
  color: PropTypes.string.isRequired,
  points: PropTypes.arrayOf(PropTypes.arrayOf(PropTypes.number)),
}
