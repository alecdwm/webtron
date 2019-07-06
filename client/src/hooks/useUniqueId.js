import { useRef } from 'react'

window.hooks_useUniqueId_uniqueIds = {}

export default function useUniqueId() {
  const idRef = useRef(null)
  if (idRef.current !== null) return idRef.current

  idRef.current = generateId()
  while (window.hooks_useUniqueId_uniqueIds[idRef.current] !== undefined) {
    idRef.current = generateId()
  }
  window.hooks_useUniqueId_uniqueIds[idRef.current] = true

  return idRef.current
}

function generateId() {
  return parseInt(Math.random() * Math.pow(10, 8)).toString(16)
}
