import type {
  ComponentPropsWithoutRef,
  ElementType,
  PropsWithChildren,
} from 'react'
import { cn } from '@/lib/cn'

type CardPadding = 'none' | 'sm' | 'md' | 'lg'
type CardTone = 'default' | 'strong' | 'subtle'

type SurfaceCardProps<T extends ElementType> = PropsWithChildren<{
  as?: T
  padding?: CardPadding
  tone?: CardTone
  className?: string
}> &
  Omit<ComponentPropsWithoutRef<T>, 'as' | 'className'>

const paddingClassMap: Record<CardPadding, string> = {
  none: '',
  sm: 'p-4',
  md: 'p-5',
  lg: 'p-6',
}

const toneClassMap: Record<CardTone, string> = {
  default: 'border-white/10 bg-surface/80',
  strong: 'border-white/15 bg-surface/85',
  subtle: 'border-white/5 bg-surface/60',
}

export function SurfaceCard<T extends ElementType = 'section'>({
  as,
  padding = 'md',
  tone = 'default',
  className,
  children,
  ...rest
}: SurfaceCardProps<T>) {
  const Component = as ?? 'section'

  return (
    <Component
      className={cn(
        'rounded-3xl border shadow-[0_35px_110px_rgba(0,0,0,0.35)] backdrop-blur',
        toneClassMap[tone],
        paddingClassMap[padding],
        className
      )}
      {...rest}
    >
      {children}
    </Component>
  )
}
