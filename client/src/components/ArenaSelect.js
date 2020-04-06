import { join } from 'actions'
import MenuButton from 'components/MenuButton'
import useArenaListPolling from 'hooks/useArenaListPolling'
import useClassName from 'hooks/useClassName'
import usePreloadImages from 'hooks/usePreloadImages'
import useStore from 'hooks/useStore'
import useStoreDispatch from 'hooks/useStoreDispatch'
import React, { useCallback } from 'react'
import gridbikeImages from 'utils/gridbikeImages'

import styles from './ArenaSelect.module.css'

export default function ArenaSelect() {
  usePreloadImages(Object.values(gridbikeImages))
  useArenaListPolling()

  const { arenaList } = useStore()
  const dispatch = useStoreDispatch()

  const joinArena = useCallback(({ currentTarget }) => dispatch(join(currentTarget.getAttribute('data-id'))), [
    dispatch,
  ])

  const ArenaSelect = useClassName(styles.arenaSelect)
  const NewArenaButton = useClassName(styles.newArenaButton, MenuButton)
  const ArenaList = useClassName(styles.arenaList)
  const Arena = useClassName(styles.arena)
  const ArenaName = useClassName(styles.arenaName)
  const ArenaSpace = useClassName(styles.arenaSpace)
  const ArenaPlayers = useClassName(styles.arenaPlayers)
  const ArenaPlayer = useClassName(styles.arenaPlayer, 'img')
  const JoinButton = useClassName(styles.joinButton, MenuButton)

  return (
    <ArenaSelect>
      <NewArenaButton onClick={joinArena}>NEW ARENA</NewArenaButton>
      <ArenaList>
        {arenaList.map((arena) => (
          <Arena key={arena.id}>
            <ArenaName>{arena.name}</ArenaName>
            <ArenaSpace />
            <ArenaPlayers>
              {Object.entries(arena.players).map(([playerId, player]) => (
                <ArenaPlayer key={playerId} src={gridbikeImages[player.color]} />
              ))}
              {[...Array(arena.max_players - Object.keys(arena.players).length)].map((_, index) => (
                <ArenaPlayer key={index} src={gridbikeImages['dark']} />
              ))}
            </ArenaPlayers>
            <JoinButton data-id={arena.id} onClick={joinArena}>
              JOIN
            </JoinButton>
          </Arena>
        ))}
      </ArenaList>
    </ArenaSelect>
  )
}
