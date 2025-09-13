// Keep this as a module file to avoid polluting globals
export {};

declare module "next-auth" {
  interface Session {
    /** JWT from backend we attach to the session */
    backendJwt?: string;
  }
}

declare module "next-auth/jwt" {
  interface JWT {
    backendJwt?: string;
  }
}

