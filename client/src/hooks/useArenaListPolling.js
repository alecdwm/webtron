import { getArenaList } from 'actions'
import useStoreDispatch from 'hooks/useStoreDispatch'
import { useEffect } from 'react'

export default function useArenaListPolling() {
  const dispatch = useStoreDispatch()

  useEffect(() => {
    let timeout = null
    const getArenaListTimeout = () => {
      dispatch(getArenaList())
      timeout = window.setTimeout(getArenaListTimeout, 2000)
    }
    getArenaListTimeout()

    return () => window.clearTimeout(timeout)
  }, [dispatch])
}
