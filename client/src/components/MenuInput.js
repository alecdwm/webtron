import useClassName, { resolveClassName } from 'hooks/useClassName'
import useCursorBlink from 'hooks/useCursorBlink'
import useUniqueId from 'hooks/useUniqueId'
import PropTypes from 'prop-types'
import React, { useCallback, useLayoutEffect, useRef, useState } from 'react'

import styles from './MenuInput.module.css'

export default function MenuInput({ className, focusOnMount, onChange, onSubmit, value, ...passProps }) {
  const inputRef = useRef(null)
  useLayoutEffect(() => {
    if (!focusOnMount) return
    if (inputRef.current === null) return
    window.setTimeout(() => inputRef.current.focus(), 10)
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

  const uniqueId = useUniqueId()

  const MenuInput = useClassName(styles.menuInput)
  const InputWrapper = useClassName([styles.inputWrapper, cursorBlink && styles.cursorBlink, focused && styles.focused])
  const InvisibleText = useClassName(styles.invisibleText, 'span')
  const Input = useClassName(styles.input, 'input')

  return (
    <MenuInput className={resolveClassName(className)}>
      <InputWrapper htmlFor={uniqueId} className={value === '' && styles.noName}>
        <InvisibleText>{value || '_'}</InvisibleText>
        <Input
          ref={inputRef}
          spellCheck={false}
          id={uniqueId}
          value={value}
          onChange={handleChange}
          onKeyDown={handleKeyDown}
          onSelect={handleSelect}
          onFocus={handleFocus}
          onBlur={handleBlur}
          {...passProps}
        />
      </InputWrapper>
    </MenuInput>
  )
}
MenuInput.propTypes = {
  className: PropTypes.oneOfType([PropTypes.string, PropTypes.array]),
  focusOnMount: PropTypes.bool,
  onChange: PropTypes.func,
  onSubmit: PropTypes.func,
  value: PropTypes.string,
}
