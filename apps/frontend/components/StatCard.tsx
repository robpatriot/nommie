import type { ReactNode } from 'react'
import { cn } from '@/lib/cn'

type StatCardProps = {
  label: ReactNode
  value: ReactNode
  description?: ReactNode
  align?: 'center' | 'start'
  className?: string
  valueClassName?: string
  descriptionClassName?: string
}

export function StatCard({
  label,
  value,
  description,
  align = 'center',
  className,
  valueClassName,
  descriptionClassName,
}: StatCardProps) {
  return (
    <div
      className={cn(
        'flex h-full flex-col gap-1 rounded-2xl border border-border/60 bg-card px-4 py-3 text-muted-foreground shadow-inner shadow-shadow/5',
        align === 'center'
          ? 'items-center text-center'
          : 'items-start text-left',
        className
      )}
    >
      <p className="text-[11px] font-semibold uppercase tracking-[0.4em] text-muted-foreground">
        {label}
      </p>
      <p
        className={cn('text-2xl font-semibold text-foreground', valueClassName)}
      >
        {value}
      </p>
      {description ? (
        <p
          className={cn(
            'text-xs text-muted-foreground/90',
            descriptionClassName
          )}
        >
          {description}
        </p>
      ) : null}
    </div>
  )
}
