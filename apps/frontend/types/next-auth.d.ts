// Keep this as a module file to avoid polluting globals
export {};

declare module "next-auth/jwt" {
  interface JWT {
    email?: string;
    googleSub?: string;
    name?: string;
  }
}

