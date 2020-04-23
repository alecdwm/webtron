import { useEffect, useRef } from 'react'

export default function useRequestAnimationFrame(callback) {
  const callbackRef = useRef(null)
  callbackRef.current = callback

  useEffect(() => {
    const handleAnimationFrame = () => {
      if (callbackRef.current !== callback) return
      callback()
      requestAnimationFrame(handleAnimationFrame)
    }
    requestAnimationFrame(handleAnimationFrame)
  }, [callback])

  useEffect(() => () => (callbackRef.current = null), [])
}
