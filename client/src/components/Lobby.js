import { MAX_PLAYER_NAME_LENGTH } from 'components/MainMenu'
import MenuButton from 'components/MenuButton'
import MenuInput from 'components/MenuInput'
import useBooleanState from 'hooks/useBooleanState'
import useClassName from 'hooks/useClassName'
import useLobbyDataPolling from 'hooks/useLobbyDataPolling'
import usePreloadImages from 'hooks/usePreloadImages'
import gridbikeBlue from 'img/gridbike-blue.png'
import gridbikeGreen from 'img/gridbike-green.png'
import gridbikeOrange from 'img/gridbike-orange.png'
import gridbikePurple from 'img/gridbike-purple.png'
import gridbikeRed from 'img/gridbike-red.png'
import gridbikeWhite from 'img/gridbike-white.png'
import React, { useCallback, useState } from 'react'

import styles from './Lobby.module.css'

const gridbikeImages = {
  blue: gridbikeBlue,
  green: gridbikeGreen,
  orange: gridbikeOrange,
  purple: gridbikePurple,
  red: gridbikeRed,
  white: gridbikeWhite,
}

const MAX_GAME_NAME_LENGTH = MAX_PLAYER_NAME_LENGTH

export default function Lobby({ store: { playerId, games, players }, disconnect, send }) {
  usePreloadImages([gridbikeBlue, gridbikeGreen, gridbikeOrange, gridbikePurple, gridbikeRed, gridbikeWhite])

  useLobbyDataPolling(send)

  const [createGameMenuIsOpen, openCreateGameMenu, closeCreateGameMenu] = useBooleanState(false)
  const currentGame = Object.values(games.byId).find((game) => game && game.players && game.players.includes(playerId))

  const createGame = useCallback(
    (name) => {
      send({ Lobby: { CreateGame: name } })
      send({ Lobby: 'FetchLobbyData' })
    },
    [send],
  )
  const joinGame = useCallback(
    (name) => {
      send({ Lobby: { JoinGame: name } })
      send({ Lobby: 'FetchLobbyData' })
    },
    [send],
  )
  const leaveGame = useCallback(() => {
    send({ Lobby: 'LeaveGame' })
    send({ Lobby: 'FetchLobbyData' })
  }, [send])

  if (currentGame)
    return (
      <GameLobby currentGame={currentGame} playerId={playerId} games={games} players={players} leaveGame={leaveGame} />
    )

  if (createGameMenuIsOpen)
    return (
      <CreateGameMenu
        playerId={playerId}
        players={players}
        createGame={createGame}
        closeCreateGameMenu={closeCreateGameMenu}
      />
    )

  return (
    <MainLobby
      playerId={playerId}
      games={games}
      players={players}
      joinGame={joinGame}
      openCreateGameMenu={openCreateGameMenu}
      disconnect={disconnect}
    />
  )
}

function MainLobby({ games, players, joinGame, openCreateGameMenu, disconnect }) {
  const handleJoinClick = useCallback((event) => joinGame(event.currentTarget.getAttribute('data-id')), [joinGame])

  const MainLobby = useClassName(styles.mainLobby)
  const Space = useClassName(styles.space)
  const Title = useClassName(styles.title)
  const GameList = useClassName(styles.mainLobbyGameList)
  const GameItem = useClassName(styles.mainLobbyGameItem)
  const GameItemName = useClassName(styles.mainLobbyGameItemName)
  const GameItemPlayers = useClassName(styles.mainLobbyGameItemPlayers)
  const JoinButton = useClassName(styles.mainLobbyJoinButton, MenuButton)

  const CreateButton = useClassName(styles.createButton, MenuButton)
  const DisconnectButton = useClassName(styles.disconnectButton, MenuButton)

  const noGames = games.allIds.length < 1

  return (
    <MainLobby>
      <Space />
      <Title>GAMES</Title>
      <Space />
      <GameList>
        {noGames ? 'No Games Found' : null}
        {games.allIds.map((gameId) => (
          <GameItem key={gameId}>
            <GameItemName>{games.byId[gameId].name}</GameItemName>
            <GameItemPlayers>
              {games.byId[gameId].players.map((playerId) => (
                <img
                  style={{ transform: 'rotate(270deg)' }}
                  key={playerId}
                  src={gridbikeImages[players.byId[playerId].color]}
                />
              ))}
            </GameItemPlayers>
            <GameItemPlayers>
              {games.byId[gameId].players.length} / {games.byId[gameId].max_players}
            </GameItemPlayers>
            <JoinButton data-id={gameId} onClick={handleJoinClick}>
              JOIN
            </JoinButton>
          </GameItem>
        ))}
      </GameList>
      <Space />
      <CreateButton onClick={openCreateGameMenu}>CREATE GAME</CreateButton>
      <DisconnectButton onClick={disconnect}>DISCONNECT</DisconnectButton>
      <Space />
    </MainLobby>
  )
}

function GameLobby({ currentGame, playerId: currentPlayerId, players, leaveGame }) {
  const GameLobby = useClassName(styles.gameLobby)
  const Space = useClassName(styles.space)
  const Title = useClassName(styles.title)
  const LeaveButton = useClassName(styles.leaveButton, MenuButton)

  return (
    <GameLobby>
      <Space />
      <Title>{currentGame.name}</Title>
      <Space />
      <div>players:</div>
      {currentGame.players.map((playerId) => (
        <div key={playerId}>
          {players.byId[playerId].name}
          {playerId === currentPlayerId ? ' (you)' : ''}
        </div>
      ))}
      <Space />
      <LeaveButton onClick={leaveGame}>LEAVE</LeaveButton>
      <Space />
    </GameLobby>
  )
}

function CreateGameMenu({ playerId, players, createGame, closeCreateGameMenu }) {
  const [gameName, setGameName] = useState('')
  const handleCreateClick = useCallback(() => {
    const name = gameName || `${players.byId[playerId].name.slice(0, MAX_GAME_NAME_LENGTH - "'s game".length)}'s game`
    createGame(name)
  }, [gameName, players, playerId, createGame])

  const handleGameNameChange = useCallback(
    (name) => {
      setGameName(name.slice(0, MAX_GAME_NAME_LENGTH).replace(/ /g, '_').toLowerCase())
    },
    [setGameName],
  )

  const CreateGameMenu = useClassName(styles.createGameMenu)
  const Space = useClassName(styles.space)
  const Title = useClassName(styles.title)
  const GameName = useClassName(styles.gameName)
  const GameNameLabel = useClassName(styles.gameNameLabel)
  const GameNameInput = useClassName(styles.gameNameInput, MenuInput)
  const CreateButton = useClassName(styles.createButton, MenuButton)
  const BackButton = useClassName(styles.backButton, MenuButton)

  return (
    <CreateGameMenu>
      <Space />
      <Title>CREATE GAME</Title>
      <Space />
      <GameName>
        <GameNameLabel>NAME:</GameNameLabel>
        <GameNameInput focusOnMount value={gameName} onChange={handleGameNameChange} onSubmit={handleCreateClick} />
      </GameName>
      <Space />
      <CreateButton onClick={handleCreateClick}>CREATE</CreateButton>
      <BackButton onClick={closeCreateGameMenu}>BACK</BackButton>
      <Space />
    </CreateGameMenu>
  )
}
