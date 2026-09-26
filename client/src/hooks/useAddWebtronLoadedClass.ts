import { useEffect } from 'react'

export default function useAddWebtronLoadedClass() {
  useEffect(() => {
    document.getElementById('webtron')?.classList.add('loaded')
  }, [])
}
