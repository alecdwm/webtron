import { useCallback } from 'react'

import { join } from '@/actions'
import MenuButton from '@/components/MenuButton'
import useArenaListPolling from '@/hooks/useArenaListPolling'
import usePreloadImages from '@/hooks/usePreloadImages'
import useStore from '@/hooks/useStore'
import useStoreDispatch from '@/hooks/useStoreDispatch'
import lightcycleImages from '@/utils/lightcycleImages'

import styles from './ArenaSelect.module.css'

export default function ArenaSelect() {
  usePreloadImages(Object.values(lightcycleImages))
  useArenaListPolling()

  const { arenaList } = useStore()
  const dispatch = useStoreDispatch()

  const joinArena = useCallback(
    ({ currentTarget }) => dispatch(join(currentTarget.getAttribute('data-id'))),
    [dispatch],
  )

  return (
    <div className={styles.arenaSelect}>
      <MenuButton className={styles.newArenaButton} onClick={joinArena}>
        NEW ARENA
      </MenuButton>
      <div className={styles.arenaList}>
        {arenaList.map((arena) => (
          <div key={arena.id} className={styles.arena}>
            <div className={styles.arenaName}>{arena.name}</div>
            <div className={styles.arenaSpace} />
            <div className={styles.arenaPlayers}>
              {Object.entries(arena.players).map(([playerId, player]) => (
                <img key={playerId} className={styles.arenaPlayer} src={lightcycleImages[player.color]} />
              ))}
              {[...Array(arena.max_players - Object.keys(arena.players).length)].map((_, index) => (
                <img key={index} className={styles.arenaPlayer} src={lightcycleImages['dark']} />
              ))}
            </div>
            <MenuButton className={styles.joinButton} data-id={arena.id} onClick={joinArena}>
              JOIN
            </MenuButton>
          </div>
        ))}
      </div>
    </div>
  )
}
