import '@/index.css'

import { unlockAudio } from '@/audio/sound'
import SoundToggle from '@/components/SoundToggle'
import Webtron from '@/components/Webtron'
import useAddWebtronLoadedClass from '@/hooks/useAddWebtronLoadedClass'
import useEventListener from '@/hooks/useEventListener'
import usePreventWebtronContextMenu from '@/hooks/usePreventWebtronContextMenu'
import { StoreContext } from '@/hooks/useStore'
import { StoreDispatchContext } from '@/hooks/useStoreDispatch'
import useThunkReducer from '@/hooks/useThunkReducer'
import reducers from '@/reducers'

const initialStore = reducers(undefined, { type: 'INIT_STORE' })

export default function App() {
  usePreventWebtronContextMenu()
  useAddWebtronLoadedClass()
  useEventListener('pointerdown', unlockAudio)
  useEventListener('pointerup', unlockAudio)
  useEventListener('touchend', unlockAudio)
  useEventListener('click', unlockAudio)
  useEventListener('keydown', unlockAudio)

  const [store, dispatch] = useThunkReducer(reducers, initialStore)

  return (
    <StoreContext.Provider value={store}>
      <StoreDispatchContext.Provider value={dispatch}>
        <Webtron />
        <SoundToggle />
      </StoreDispatchContext.Provider>
    </StoreContext.Provider>
  )
}
