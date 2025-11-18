'use client'

import { ReactNode } from 'react'
import { cn } from '@/lib/cn'

interface PageHeroProps {
  intro: ReactNode
  aside?: ReactNode
  footer?: ReactNode
  className?: string
  introClassName?: string
  asideClassName?: string
  footerClassName?: string
}

export function PageHero({
  intro,
  aside,
  footer,
  className,
  introClassName,
  asideClassName,
  footerClassName,
}: PageHeroProps) {
  return (
    <section
      className={cn(
        'rounded-3xl border border-white/10 bg-surface/80 p-6 shadow-[0_45px_120px_rgba(0,0,0,0.35)] backdrop-blur',
        className
      )}
    >
      <div className="flex flex-col gap-6 lg:flex-row lg:items-start lg:justify-between">
        <div className={cn('flex flex-col gap-4 lg:flex-1', introClassName)}>
          {intro}
        </div>
        {aside ? (
          <div
            className={cn(
              'flex w-full flex-col gap-4 lg:max-w-sm',
              asideClassName
            )}
          >
            {aside}
          </div>
        ) : null}
      </div>
      {footer ? (
        <div className={cn('mt-4', footerClassName)}>{footer}</div>
      ) : null}
    </section>
  )
}
