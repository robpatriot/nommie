# Docker Environment Configuration

This directory contains environment configuration files for different deployment scenarios.

## ⚠️ Security Notice

**Never commit `.env` files to git!** They contain sensitive secrets. Only `.env.example` files are tracked in version control.

## 🔒 Postgres TLS

All Postgres instances (dev-db and local-prod) use TLS encryption by default. See [`postgres-tls/README.md`](postgres-tls/README.md) for:
- How to generate your private CA (one-time setup)
- How to build the TLS-enabled Postgres image
- Backend TLS configuration
- Certificate expiry and rotation

## Setup Instructions

### For Local Development (`dev-db/`)

1. Copy the example file:
   ```bash
   cp docker/dev-db/db.env.example docker/dev-db/db.env
   ```

2. Edit `docker/dev-db/db.env` with your local development values.

### For Local Production-like Setup (`local-prod/`)

1. Copy all example files:
   ```bash
   cp docker/local-prod/backend.env.example docker/local-prod/backend.env
   cp docker/local-prod/frontend.env.example docker/local-prod/frontend.env
   cp docker/local-prod/db.env.example docker/local-prod/db.env
   cp docker/local-prod/caddy.env.example docker/local-prod/caddy.env
   ```

2. Edit each `.env` file with your actual values:
   - **backend.env**: Set `BACKEND_JWT_SECRET` (generate with `openssl rand -hex 32`)
   - **frontend.env**: Set `AUTH_SECRET`, `AUTH_GOOGLE_ID`, `AUTH_GOOGLE_SECRET`, and URLs
   - **db.env**: Set database passwords (generate with `openssl rand -hex 16`)
   - **caddy.env**: Set `CADDY_DOMAIN` (your domain name) and `CF_API_TOKEN` (Cloudflare API token for TLS)

## Generating Secure Secrets

### JWT/Auth Secrets (32 bytes = 64 hex characters)
```bash
openssl rand -hex 32
```

### Database Passwords (16 bytes = 32 hex characters)
```bash
openssl rand -hex 16
```

## Important Notes

- **JWT Secrets**: `BACKEND_JWT_SECRET` (backend) and `AUTH_SECRET` (NextAuth/frontend) are separate secrets serving different purposes:
  - `BACKEND_JWT_SECRET`: Used by the backend to sign/verify API authentication tokens
  - `AUTH_SECRET`: Used by NextAuth to sign/verify frontend session tokens
  - These do NOT need to match - they are independent systems
- **Database Passwords**: Use strong, randomly generated passwords in production
- **CORS Origins**: Only include trusted domains in `CORS_ALLOWED_ORIGINS`
- **Google OAuth**: Get credentials from [Google Cloud Console](https://console.cloud.google.com/apis/credentials)

## File Structure

```
docker/
├── dev-db/
│   ├── db.env.example          # Example for local dev database
│   └── db.env                  # Your actual dev config (gitignored)
└── local-prod/
    ├── backend.env.example      # Example for backend
    ├── backend.env              # Your actual backend config (gitignored)
    ├── frontend.env.example     # Example for frontend
    ├── frontend.env             # Your actual frontend config (gitignored)
    ├── db.env.example           # Example for production database
    ├── db.env                   # Your actual prod config (gitignored)
    ├── caddy.env.example        # Example for Caddy reverse proxy
    └── caddy.env                # Your actual Caddy config (gitignored)
```



