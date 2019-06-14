import { useEffect } from 'react'

export default function usePreventWebtronContextMenu() {
  useEffect(() => {
    const webtron = document.getElementById('webtron')
    const preventDefault = e => e.preventDefault()

    webtron.addEventListener('contextmenu', preventDefault)

    return () => webtron.removeEventListener('contextmenu', preventDefault)
  }, [])
}
