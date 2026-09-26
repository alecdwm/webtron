import { useEffect } from 'react'

import { getArenaList } from '@/actions'
import useStoreDispatch from '@/hooks/useStoreDispatch'

export default function useArenaListPolling() {
  const dispatch = useStoreDispatch()

  useEffect(() => {
    let timeout: number | undefined
    const getArenaListTimeout = () => {
      dispatch(getArenaList())
      timeout = window.setTimeout(getArenaListTimeout, 2000)
    }
    getArenaListTimeout()

    return () => window.clearTimeout(timeout)
  }, [dispatch])
}
