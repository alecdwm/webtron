import { useCallback, useEffect, useRef, useState } from 'react'

export default function useCursorBlink(): [boolean, () => void] {
  const [cursorBlink, setCursorBlink] = useState(true)
  const cursorBlinkTimeout = useRef<number | null>(null)

  useEffect(() => () => window.clearTimeout(cursorBlinkTimeout.current ?? undefined), [])

  const resetCursorBlink = useCallback(() => {
    window.clearTimeout(cursorBlinkTimeout.current ?? undefined)
    setCursorBlink(false)
    cursorBlinkTimeout.current = window.setTimeout(() => setCursorBlink(true), 600)
  }, [])

  return [cursorBlink, resetCursorBlink]
}
