import { useStoreApi } from '@/hooks/useStore'
import type { Dispatch } from '@/types'

export default function useStoreDispatch(): Dispatch {
  return useStoreApi().dispatch
}
