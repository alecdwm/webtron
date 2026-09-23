import { type ComponentProps, type MouseEvent, useCallback } from 'react'

import { playClick, playHover } from '@/audio/sound'
import resolveClassName, { type ClassName } from '@/utils/resolveClassName'

import styles from './MenuButton.module.css'

type MenuButtonProps = Omit<ComponentProps<'button'>, 'className'> & {
  className?: ClassName
}

export default function MenuButton({ className, onClick, onMouseEnter, ...passProps }: MenuButtonProps) {
  const handleClick = useCallback(
    (event: MouseEvent<HTMLButtonElement>) => {
      playClick()
      onClick?.(event)
    },
    [onClick],
  )
  const handleMouseEnter = useCallback(
    (event: MouseEvent<HTMLButtonElement>) => {
      playHover()
      onMouseEnter?.(event)
    },
    [onMouseEnter],
  )

  return (
    <button
      className={resolveClassName([styles.menuButton, className])}
      onClick={handleClick}
      onMouseEnter={handleMouseEnter}
      {...passProps}
    />
  )
}
