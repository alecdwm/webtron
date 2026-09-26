import { useEffect } from 'react'

export default function usePreventWebtronContextMenu() {
  useEffect(() => {
    const webtron = document.getElementById('webtron')
    if (!webtron) return
    const preventDefault = (event: Event) => event.preventDefault()
    webtron.addEventListener('contextmenu', preventDefault)

    return () => webtron.removeEventListener('contextmenu', preventDefault)
  }, [])
}
