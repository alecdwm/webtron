import type { ComponentProps } from 'react'

import resolveClassName, { type ClassName } from '@/utils/resolveClassName'

import styles from './MenuButton.module.css'

type MenuButtonProps = Omit<ComponentProps<'button'>, 'className'> & {
  className?: ClassName
}

export default function MenuButton({ className, ...passProps }: MenuButtonProps) {
  return <button className={resolveClassName([styles.menuButton, className])} {...passProps} />
}
