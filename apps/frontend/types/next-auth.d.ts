// Keep this as a module file to avoid polluting globals
export {};

declare module "next-auth/jwt" {
  interface JWT {
    backendJwt?: string;
    email?: string;
    googleSub?: string;
    name?: string;
  }
}

