import { useEffect, useRef } from 'react'

// Starts loading the images on mount, and holds them while mounted.
export default function usePreloadImages(urls: string[]) {
  const initialUrls = useRef(urls)
  const images = useRef<HTMLImageElement[]>([])

  useEffect(() => {
    images.current = initialUrls.current.map((url) => {
      const image = new Image()
      image.src = url
      return image
    })
  }, [])
}
