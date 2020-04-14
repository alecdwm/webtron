import useAddWebtronLoadedClass from 'hooks/useAddWebtronLoadedClass'
import usePreventWebtronContextMenu from 'hooks/usePreventWebtronContextMenu'
import dynamic from 'next/dynamic'
import React from 'react'

const Webtron = dynamic(() => import('components/Webtron'), { ssr: false })

export default function IndexPage() {
  usePreventWebtronContextMenu()
  useAddWebtronLoadedClass()

  return <Webtron />
}
