import useClassName from 'hooks/useClassName'
import useStore from 'hooks/useStore'
import PropTypes from 'prop-types'
import React from 'react'
import { colorToHexString } from 'utils/colors'
import lightcycleImages from 'utils/lightcycleImages'

import styles from './Lightcycle.module.css'

export default function Lightcycle({ name, color, position = [0, 0], direction, speed, dead }) {
  const {
    arena: { width, height },
  } = useStore()

  const left = `${(position[0] / width) * 100}%`
  const bottom = `${(position[1] / height) * 100}%`
  const tagBottom = `calc(${(position[1] / height) * 100}% + 20px)`
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
      <NameTag style={{ left, bottom: tagBottom, color: colorToHexString(color) }}>{name}</NameTag>
      <Lightcycle src={lightcycleImages[color]} style={{ left, bottom, transform }} />
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
