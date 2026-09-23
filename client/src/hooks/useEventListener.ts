import { type RefObject, useEffect } from 'react'

// Listens for `event` on the element held by `targetRef`, or on `window` when
// no ref is given.
export default function useEventListener(event, callback, targetRef?: RefObject<EventTarget | null>) {
  useEffect(() => {
    const target = targetRef ? targetRef.current : window
    if (!target) return

    target.addEventListener(event, callback)

    return () => target.removeEventListener(event, callback)
  }, [event, callback, targetRef])
}
