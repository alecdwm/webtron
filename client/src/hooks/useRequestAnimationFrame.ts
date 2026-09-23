import { useEffect } from 'react'

export default function useRequestAnimationFrame(callback) {
  useEffect(() => {
    let frame = requestAnimationFrame(function handleAnimationFrame() {
      callback()
      frame = requestAnimationFrame(handleAnimationFrame)
    })

    return () => cancelAnimationFrame(frame)
  }, [callback])
}
