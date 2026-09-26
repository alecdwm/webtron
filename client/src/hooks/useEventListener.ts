import { type RefObject, useEffect } from 'react'

type EventName = keyof WindowEventMap & keyof HTMLElementEventMap

// Listens for `event` on the element held by `targetRef`, or on `window` when
// no ref is given.
export default function useEventListener<K extends EventName>(
  event: K,
  callback: (event: WindowEventMap[K]) => void,
  targetRef?: RefObject<HTMLElement | null>,
) {
  useEffect(() => {
    const target = targetRef ? targetRef.current : window
    if (!target) return

    const listener = callback as EventListener
    target.addEventListener(event, listener)

    return () => target.removeEventListener(event, listener)
  }, [event, callback, targetRef])
}
