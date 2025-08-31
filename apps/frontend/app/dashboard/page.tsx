import { auth } from '@/auth'
import { redirect } from 'next/navigation'
import { getMe, BackendApiError } from '@/lib/api'

export default async function DashboardPage() {
  const session = await auth()

  if (!session) {
    redirect('/')
  }

  let backendUser: { sub: string; email: string } | null = null
  let error: string | null = null

  try {
    backendUser = await getMe()
  } catch (err) {
    if (err instanceof BackendApiError && err.status === 401) {
      redirect('/')
    }
    error = err instanceof Error ? err.message : 'Failed to fetch user data'
  }

  return (
    <main className="min-h-screen bg-gray-50 py-12">
      <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="bg-white shadow rounded-lg p-8">
          <div className="text-center">
            <h1 className="text-3xl font-bold text-gray-900 mb-4">
              🎉 Welcome to your Dashboard!
            </h1>
            <p className="text-lg text-gray-600 mb-6">
              You are successfully authenticated and can access this protected
              page.
            </p>

            {backendUser ? (
              <div className="bg-green-50 border border-green-200 rounded-md p-4 mb-4">
                <div className="flex items-center">
                  <div className="flex-shrink-0">
                    <svg
                      className="h-5 w-5 text-green-400"
                      viewBox="0 0 20 20"
                      fill="currentColor"
                    >
                      <path
                        fillRule="evenodd"
                        d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
                        clipRule="evenodd"
                      />
                    </svg>
                  </div>
                  <div className="ml-3">
                    <p className="text-sm text-green-800">
                      <strong>Backend User:</strong> {backendUser.email} (ID:{' '}
                      {backendUser.sub})
                    </p>
                  </div>
                </div>
              </div>
            ) : error ? (
              <div className="bg-red-50 border border-red-200 rounded-md p-4 mb-4">
                <p className="text-sm text-red-800">
                  <strong>Error:</strong> {error}
                </p>
              </div>
            ) : null}

            <div className="bg-blue-50 border border-blue-200 rounded-md p-4">
              <div className="flex items-center">
                <div className="flex-shrink-0">
                  <svg
                    className="h-5 w-5 text-blue-400"
                    viewBox="0 0 20 20"
                    fill="currentColor"
                  >
                    <path
                      fillRule="evenodd"
                      d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
                      clipRule="evenodd"
                    />
                  </svg>
                </div>
                <div className="ml-3">
                  <p className="text-sm text-blue-800">
                    <strong>Frontend Session:</strong> {session.user?.email}
                  </p>
                </div>
              </div>
            </div>

            <p className="text-sm text-gray-500 mt-6">
              This page is protected by NextAuth middleware. Only authenticated
              users can access it.
            </p>
          </div>
        </div>
      </div>
    </main>
  )
}
