import type { ReactNode } from 'react'
import { cn } from '@/lib/cn'

interface PageContainerProps {
  children: ReactNode
  className?: string
}

export function PageContainer({ children, className }: PageContainerProps) {
  return (
    <main
      className={cn(
        'mx-auto flex w-full max-w-6xl flex-col gap-6 px-4 pb-12 pt-6 text-foreground sm:px-6 sm:pt-8',
        className
      )}
    >
      {children}
    </main>
  )
}
