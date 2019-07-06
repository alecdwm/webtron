import React from 'react'
import useClassName from 'hooks/useClassName'
import styles from './MenuButton.module.css'

export default function MenuButton({ ...passProps }) {
  const MenuButton = useClassName([styles.menuButton], 'button')

  return <MenuButton {...passProps} />
}
