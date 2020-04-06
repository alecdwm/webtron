import useClassName, { resolveClassName } from 'hooks/useClassName'
import PropTypes from 'prop-types'
import React from 'react'

import styles from './MenuButton.module.css'

export default function MenuButton({ className, ...passProps }) {
  const MenuButton = useClassName(styles.menuButton, 'button')

  return <MenuButton className={resolveClassName(className)} {...passProps} />
}
MenuButton.propTypes = {
  className: PropTypes.oneOfType([PropTypes.string, PropTypes.array]),
}
