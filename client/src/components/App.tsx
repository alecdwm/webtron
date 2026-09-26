import '@/index.css'

import { useState } from 'react'

import { unlockAudio } from '@/audio/sound'
import SoundToggle from '@/components/SoundToggle'
import Webtron from '@/components/Webtron'
import useAddWebtronLoadedClass from '@/hooks/useAddWebtronLoadedClass'
import useEventListener from '@/hooks/useEventListener'
import usePreventWebtronContextMenu from '@/hooks/usePreventWebtronContextMenu'
import { StoreContext } from '@/hooks/useStore'
import reducers from '@/reducers'
import createStore from '@/store/createStore'

export default function App() {
  usePreventWebtronContextMenu()
  useAddWebtronLoadedClass()
  useEventListener('pointerdown', unlockAudio)
  useEventListener('pointerup', unlockAudio)
  useEventListener('touchend', unlockAudio)
  useEventListener('click', unlockAudio)
  useEventListener('keydown', unlockAudio)

  const [store] = useState(() => createStore(reducers))

  return (
    <StoreContext.Provider value={store}>
      <Webtron />
      <SoundToggle />
    </StoreContext.Provider>
  )
}
