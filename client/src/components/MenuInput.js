import useClassName, { resolveClassName } from '/hooks/useClassName'
import useCursorBlink from '/hooks/useCursorBlink'
import useUniqueId from '/hooks/useUniqueId'
import PropTypes from 'prop-types'
import React, { useCallback, useLayoutEffect, useRef, useState } from 'react'

import styles from './MenuInput.module.css'

export default function MenuInput({ value, onChange, onSubmit, focusOnMount, className, ...passProps }) {
  const inputRef = useRef(null)
  useLayoutEffect(() => {
    if (!focusOnMount) return
    if (inputRef.current === null) return
    inputRef.current.focus()
  }, [focusOnMount])

  const [cursorBlink, resetCursorBlink] = useCursorBlink()
  const [focused, setFocused] = useState(false)

  const handleChange = useCallback(
    event => {
      resetCursorBlink()
      onChange(event.currentTarget.value)
    },
    [resetCursorBlink, onChange],
  )

  const handleKeyDown = useCallback(
    ({ key }) => {
      if (key !== 'Enter') return
      onSubmit()
    },
    [onSubmit],
  )

  const handleSelect = useCallback(event => {
    const target = event.currentTarget
    const value = target.value
    target.setSelectionRange(value.length, value.length)
  }, [])

  const handleFocus = useCallback(
    event => {
      const target = event.currentTarget
      const value = target.value
      target.setSelectionRange(value.length, value.length)
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
  value: PropTypes.string,
  onChange: PropTypes.func.isRequired,
  onSubmit: PropTypes.func.isRequired,
  focusOnMount: PropTypes.bool,
  className: PropTypes.oneOfType([PropTypes.string, PropTypes.array]),
}
