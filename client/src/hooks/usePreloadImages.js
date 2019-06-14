import { useEffect, useRef } from 'react'
import { preloadImages } from 'actions'
import useStoreDispatch from 'hooks/useStoreDispatch'

export default function usePreloadImages(urls = []) {
  const dispatch = useStoreDispatch()

  // only want to preload on first render
  const urlsRef = useRef(urls)

  useEffect(() => {
    dispatch(preloadImages(urlsRef.current))
  }, [dispatch])
}
