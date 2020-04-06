import { useEffect } from 'react'

export default function useEventListener(event, callback, element = window) {
  useEffect(() => {
    element.addEventListener(event, callback)

    return () => window.removeEventListener(event, callback)
  }, [event, callback, element])
}
