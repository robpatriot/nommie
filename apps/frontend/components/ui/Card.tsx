import type {
  ComponentPropsWithoutRef,
  ElementType,
  PropsWithChildren,
} from 'react'
import { cn } from '@/lib/cn'

type CardPadding = 'none' | 'sm' | 'md' | 'lg'
type CardTone = 'default' | 'strong' | 'subtle'

type CardProps<T extends ElementType> = PropsWithChildren<{
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
  default: 'border-border/60 bg-card/80',
  strong: 'border-border/70 bg-card/85',
  subtle: 'border-border/40 bg-card/60',
}

export function Card<T extends ElementType = 'section'>({
  as,
  padding = 'md',
  tone = 'default',
  className,
  children,
  ...rest
}: CardProps<T>) {
  const Component = as ?? 'section'

  return (
    <Component
      className={cn(
        'rounded-3xl border shadow-elevated backdrop-blur',
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
