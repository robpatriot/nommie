'use server'

import { redirect } from 'next/navigation'
import { signIn, signOut } from '@/auth'

export async function signOutAction() {
  await signOut()
  redirect('/')
}

export async function signInWithGoogleAction() {
  await signIn('google')
}
