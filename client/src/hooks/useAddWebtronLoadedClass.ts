import { useEffect } from 'react'

export default function useAddWebtronLoadedClass() {
  useEffect(() => {
    const webtron = document.getElementById('webtron')
    webtron.classList.add('loaded')
  }, [])
}
