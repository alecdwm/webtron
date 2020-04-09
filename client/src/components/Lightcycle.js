import useClassName from 'hooks/useClassName'
import PropTypes from 'prop-types'
import React from 'react'
import { colorToHexString } from 'utils/colors'
import gridbikeImages from 'utils/gridbikeImages'

import styles from './Lightcycle.module.css'

export default function Lightcycle({ name, color, position = [0, 0], direction, speed, dead }) {
  const left = position[0]
  const bottom = position[1]
  const rotationMap = {
    up: '270deg',
    down: '90deg',
    left: '180deg',
    right: '0deg',
  }
  const transform = `translate(-50%, 50%) rotate(${rotationMap[direction]})`

  const NameTag = useClassName(styles.nameTag)
  const Lightcycle = useClassName(styles.lightcycle, 'img')

  return (
    <>
      <NameTag style={{ left, bottom: bottom + 20, color: colorToHexString(color) }}>{name}</NameTag>
      <Lightcycle src={gridbikeImages[color]} style={{ left, bottom, transform }} />
    </>
  )
}
Lightcycle.propTypes = {
  name: PropTypes.string.isRequired,
  color: PropTypes.string.isRequired,
  position: PropTypes.arrayOf(PropTypes.number).isRequired,
  direction: PropTypes.string.isRequired,
  speed: PropTypes.number.isRequired,
  dead: PropTypes.bool,
}
