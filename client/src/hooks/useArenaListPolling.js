import { useEffect } from 'react'

export default function useArenaListPolling(send) {
  useEffect(() => {
    let timeout = null
    const getArenaList = () => {
      send({ GetArenaList: null })
      timeout = window.setTimeout(getArenaList, 2000)
    }
    getArenaList()

    return () => window.clearTimeout(timeout)
  }, [send])
}
