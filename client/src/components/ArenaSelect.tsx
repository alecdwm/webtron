import { useCallback } from 'react'

import { join } from '@/actions'
import MenuButton from '@/components/MenuButton'
import useArenaListPolling from '@/hooks/useArenaListPolling'
import usePreloadImages from '@/hooks/usePreloadImages'
import useStore from '@/hooks/useStore'
import useStoreDispatch from '@/hooks/useStoreDispatch'
import { colorToHexString } from '@/utils/colors'
import lightcycleImages from '@/utils/lightcycleImages'
import resolveClassName from '@/utils/resolveClassName'

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
      <div className={styles.header}>
        <div className={styles.title}>Select arena</div>
        <MenuButton className={styles.newArenaButton} onClick={joinArena}>
          + NEW ARENA
        </MenuButton>
      </div>
      <div className={styles.arenaList}>
        {arenaList.length === 0 ? (
          <div className={styles.empty}>
            No active arenas
            <br />
            Start a new one
          </div>
        ) : null}
        {arenaList.map((arena, index) => {
          const players = Object.entries(arena.players)
          const full = players.length >= arena.max_players
          return (
            <div
              key={arena.id}
              className={resolveClassName([styles.arena, full && styles.arenaFull])}
              style={{ animationDelay: `${index * 60}ms` }}
            >
              <div className={styles.arenaInfo}>
                <div className={styles.arenaName}>{arena.name}</div>
                <div className={styles.arenaMeta}>
                  <span className={arena.started ? styles.live : styles.lobby}>
                    {arena.started ? 'In progress' : 'Lobby'}
                  </span>
                  <span>
                    {players.length}/{arena.max_players}
                  </span>
                </div>
              </div>
              <div className={styles.arenaPlayers}>
                {players.map(([playerId, player]) => (
                  <img
                    key={playerId}
                    className={styles.arenaPlayer}
                    style={{ color: colorToHexString(player.color) }}
                    src={lightcycleImages[player.color]}
                    alt={player.name}
                    title={player.name}
                  />
                ))}
                {[...Array(Math.max(0, arena.max_players - players.length))].map((_, index) => (
                  <img
                    key={index}
                    className={resolveClassName([styles.arenaPlayer, styles.arenaPlayerEmpty])}
                    src={lightcycleImages['dark']}
                    alt=""
                  />
                ))}
              </div>
              <MenuButton className={styles.joinButton} data-id={arena.id} onClick={joinArena}>
                JOIN
              </MenuButton>
            </div>
          )
        })}
      </div>
    </div>
  )
}
