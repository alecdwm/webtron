import { useRef } from 'react'

const uniqueIds = {}

export default function useUniqueId() {
  const idRef = useRef(null)
  if (idRef.current !== null) return idRef.current

  idRef.current = generateId()
  while (uniqueIds[idRef.current] !== undefined) {
    idRef.current = generateId()
  }
  uniqueIds[idRef.current] = true

  return idRef.current
}

function generateId() {
  return parseInt(Math.random() * Math.pow(10, 8)).toString(16)
}
