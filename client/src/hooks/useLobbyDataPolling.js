import { useEffect } from 'react'

export default function useLobbyDataPolling(send) {
  useEffect(() => {
    let timeout = null
    const fetchLobbyData = () => {
      send({ Lobby: 'FetchLobbyData' })
      timeout = window.setTimeout(fetchLobbyData, 5000)
    }
    fetchLobbyData()

    return () => window.clearTimeout(timeout)
  }, [send])
}
