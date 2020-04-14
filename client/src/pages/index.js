import Webtron from 'components/Webtron'
import useAddWebtronLoadedClass from 'hooks/useAddWebtronLoadedClass'
import usePreventWebtronContextMenu from 'hooks/usePreventWebtronContextMenu'
import React from 'react'

export default function IndexPage() {
  usePreventWebtronContextMenu()
  useAddWebtronLoadedClass()

  return <Webtron />
}
