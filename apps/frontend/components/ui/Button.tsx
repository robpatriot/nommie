import type { ButtonHTMLAttributes } from 'react'
import { cn } from '@/lib/cn'

type ButtonVariant = 'primary' | 'outline' | 'ghost' | 'destructive'
type ButtonSize = 'sm' | 'md' | 'lg' | 'icon'

export type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: ButtonVariant
  size?: ButtonSize
}

const base =
  'inline-flex items-center justify-center gap-2 rounded-2xl font-semibold transition disabled:cursor-not-allowed disabled:opacity-50'

const variantClassMap: Record<ButtonVariant, string> = {
  primary:
    'bg-primary text-primary-foreground shadow-lg shadow-primary/30 hover:bg-primary/90',
  outline:
    'border border-border/70 bg-card text-foreground hover:border-primary/50 hover:bg-muted/40',
  ghost: 'bg-transparent text-foreground hover:bg-muted/40',
  destructive:
    'bg-destructive text-destructive-foreground shadow-lg shadow-destructive/30 hover:bg-destructive/90',
}

const sizeClassMap: Record<ButtonSize, string> = {
  sm: 'px-3 py-1.5 text-sm',
  md: 'px-4 py-2 text-sm',
  lg: 'px-5 py-3 text-base',
  icon: 'h-9 w-9 p-0',
}

export function Button({
  variant = 'primary',
  size = 'md',
  className,
  type,
  ...props
}: ButtonProps) {
  return (
    <button
      type={type ?? 'button'}
      className={cn(
        base,
        variantClassMap[variant],
        sizeClassMap[size],
        className
      )}
      {...props}
    />
  )
}
