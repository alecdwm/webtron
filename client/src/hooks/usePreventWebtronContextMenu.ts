import { useCallback, useEffect } from 'react'

export default function usePreventWebtronContextMenu() {
  const preventDefault = useCallback((e) => e.preventDefault(), [])

  useEffect(() => {
    const webtron = document.getElementById('webtron')
    webtron.addEventListener('contextmenu', preventDefault)

    return () => webtron.removeEventListener('contextmenu', preventDefault)
  }, [preventDefault])
}
