'use client'

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from 'react'

export type HeaderCrumb = {
  label: string
  href?: string
}

type HeaderBreadcrumbContextValue = {
  crumbs: HeaderCrumb[]
  setCrumbs: (crumbs: HeaderCrumb[]) => void
}

const HeaderBreadcrumbContext = createContext<
  HeaderBreadcrumbContextValue | undefined
>(undefined)

export function HeaderBreadcrumbProvider({
  children,
}: {
  children: React.ReactNode
}) {
  const [crumbs, setCrumbs] = useState<HeaderCrumb[]>([])
  const value = useMemo(
    () => ({
      crumbs,
      setCrumbs,
    }),
    [crumbs]
  )

  return (
    <HeaderBreadcrumbContext.Provider value={value}>
      {children}
    </HeaderBreadcrumbContext.Provider>
  )
}

export function useHeaderBreadcrumbs() {
  const context = useContext(HeaderBreadcrumbContext)
  if (!context) {
    throw new Error(
      'useHeaderBreadcrumbs must be used within HeaderBreadcrumbProvider'
    )
  }
  return context
}

export function BreadcrumbSetter({ crumbs }: { crumbs: HeaderCrumb[] }) {
  const { setCrumbs } = useHeaderBreadcrumbs()
  const stableCrumbs = useMemo(() => crumbs, [crumbs])

  const reset = useCallback(() => {
    setCrumbs([])
  }, [setCrumbs])

  useEffect(() => {
    setCrumbs(stableCrumbs)
    return () => {
      reset()
    }
  }, [stableCrumbs, setCrumbs, reset])

  return null
}
