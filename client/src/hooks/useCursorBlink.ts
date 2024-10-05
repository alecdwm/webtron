import { useCallback, useEffect, useRef, useState } from 'react'

export default function useCursorBlink() {
  const [cursorBlink, setCursorBlink] = useState(true)
  const cursorBlinkTimeout = useRef()

  // avoid setState on unmounted components
  useEffect(() => () => (cursorBlinkTimeout.current = null), [])

  const resetCursorBlink = useCallback(() => {
    if (cursorBlinkTimeout.current) {
      window.clearTimeout(cursorBlinkTimeout.current)
    }
    setCursorBlink(false)
    cursorBlinkTimeout.current = window.setTimeout(() => {
      // avoid setState on unmounted components
      if (cursorBlinkTimeout.current === null) return

      setCursorBlink(true)
    }, 600)
  }, [])

  return [cursorBlink, resetCursorBlink]
}
