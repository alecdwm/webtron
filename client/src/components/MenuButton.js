import useClassName from 'hooks/useClassName'
import React from 'react'

import styles from './MenuButton.module.css'

export default function MenuButton({ ...passProps }) {
  const MenuButton = useClassName([styles.menuButton], 'button')

  return <MenuButton {...passProps} />
}
