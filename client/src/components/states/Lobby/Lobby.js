import React, { useRef, useEffect, useCallback } from 'react'
import useClassName from 'hooks/useClassName'
// import useStoreDispatch from 'hooks/useStoreDispatch'
import styles from './Lobby.module.css'

export default function Lobby({ store: { gamesList }, send }) {
  const createGameInputRef = useRef(null)
  // const dispatch = useStoreDispatch()
  // dispatch(setGameState('InGame'))

  useEffect(() => send('ListGames'), [send])

  const joinGame = useCallback(event => send({ JoinGame: event.currentTarget.getAttribute('data-id') }), [send])
  const createGame = useCallback(() => send({ CreateGame: createGameInputRef.current.value }), [send])
  const devRefresh = useCallback(() => send('ListGames'), [send])

  const GameList = useClassName(styles.gameList)
  const GameItem = useClassName(styles.gameItem)
  const GameItemName = useClassName(styles.gameItemName)
  const GameItemPlayers = useClassName(styles.gameItemPlayers)
  const JoinButton = useClassName(styles.joinButton)

  const CreateGame = useClassName(styles.createGame)
  const CreateGameInput = useClassName(styles.createGameInput, 'input')
  const CreateButton = useClassName(styles.createButton)

  return (
    <>
      <h2>Games</h2>
      <button onClick={devRefresh}>[dev] refresh</button>
      <GameList>
        {gamesList.map(game => (
          <GameItem key={game.id}>
            <GameItemName>{game.name}</GameItemName>
            <GameItemPlayers>{game.players.length} / 4</GameItemPlayers>
            <JoinButton data-id={game.id} onClick={joinGame}>
              Join
            </JoinButton>
          </GameItem>
        ))}
      </GameList>
      <CreateGame>
        <h2>Create Game</h2>
        <CreateGameInput ref={createGameInputRef} />
        <CreateButton onClick={createGame}>Create</CreateButton>
      </CreateGame>
    </>
  )
}
