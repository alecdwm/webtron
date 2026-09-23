import { type ComponentProps, useCallback, useId, useLayoutEffect, useRef, useState } from 'react'

import useCursorBlink from '@/hooks/useCursorBlink'
import resolveClassName, { type ClassName } from '@/utils/resolveClassName'

import styles from './MenuInput.module.css'

type MenuInputProps = Omit<ComponentProps<'input'>, 'className' | 'onChange' | 'onSubmit' | 'value'> & {
  className?: ClassName
  focusOnMount?: boolean
  onChange?: (value: string) => void
  onSubmit?: () => void
  value?: string
}

export default function MenuInput({
  className,
  focusOnMount,
  onChange,
  onSubmit,
  value,
  ...passProps
}: MenuInputProps) {
  const inputRef = useRef<HTMLInputElement>(null)
  useLayoutEffect(() => {
    if (!focusOnMount) return
    if (inputRef.current === null) return
    window.setTimeout(() => inputRef.current?.focus(), 10)
  }, [focusOnMount])

  const [cursorBlink, resetCursorBlink] = useCursorBlink()
  const [focused, setFocused] = useState(false)

  const handleChange = useCallback(
    (event) => {
      resetCursorBlink()
      if (typeof onChange !== 'function') return
      onChange(event.currentTarget.value)
    },
    [resetCursorBlink, onChange],
  )

  const handleKeyDown = useCallback(
    ({ key }) => {
      if (key !== 'Enter' && key !== 'Return') return
      if (typeof onSubmit !== 'function') return
      onSubmit()
    },
    [onSubmit],
  )

  const handleSelect = useCallback(({ currentTarget }) => {
    setTimeout(() => {
      const value = currentTarget.value
      currentTarget.setSelectionRange(value.length, value.length)
    }, 0)
  }, [])

  const handleFocus = useCallback(
    ({ currentTarget }) => {
      setTimeout(() => {
        const value = currentTarget.value
        currentTarget.setSelectionRange(value.length, value.length)
      }, 0)
      resetCursorBlink()
      setFocused(true)
    },
    [resetCursorBlink, setFocused],
  )
  const handleBlur = useCallback(() => setFocused(false), [setFocused])

  const inputId = useId()

  return (
    <div className={resolveClassName([styles.menuInput, className])}>
      <label
        htmlFor={inputId}
        className={resolveClassName([
          styles.inputWrapper,
          cursorBlink && styles.cursorBlink,
          focused && styles.focused,
          value === '' && styles.noName,
        ])}
      >
        <span className={styles.invisibleText}>{value || '_'}</span>
        <input
          ref={inputRef}
          className={styles.input}
          spellCheck={false}
          id={inputId}
          value={value}
          onChange={handleChange}
          onKeyDown={handleKeyDown}
          onSelect={handleSelect}
          onFocus={handleFocus}
          onBlur={handleBlur}
          {...passProps}
        />
      </label>
    </div>
  )
}
