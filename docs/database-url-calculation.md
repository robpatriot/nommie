# Database URL Construction

## Purpose

Defines how the backend constructs the PostgreSQL connection URL from environment variables.

## Environment Variables

### Connection

POSTGRES_HOST (default: localhost)  
POSTGRES_PORT (default: 5432)

### Database Names

PROD_DB — production database name  
TEST_DB — test database name (must end with `_test`)

### Credentials

APP_DB_USER  
APP_DB_PASSWORD

NOMMIE_OWNER_USER  
NOMMIE_OWNER_PASSWORD

Application credentials are used for normal runtime access.  
Owner credentials are used only for migrations.

### TLS Configuration

POSTGRES_SSL_MODE (default: verify-full)

Valid values:

disable  
require  
verify-ca  
verify-full

POSTGRES_SSL_ROOT_CERT — absolute path to CA certificate.

Required when SSL mode is not `disable`.

## URL Format

Base format:

postgresql://{username}:{password}@{host}:{port}/{database}

### With SSL

When `POSTGRES_SSL_MODE != disable`:

postgresql://{username}:{password}@{host}:{port}/{database}?sslmode={ssl_mode}&sslrootcert={root_cert}

### Without SSL

When `POSTGRES_SSL_MODE = disable`:

postgresql://{username}:{password}@{host}:{port}/{database}

## Username and Password Encoding

Credentials are percent-encoded using RFC 3986 userinfo rules.

Allowed characters:

a–z  
A–Z  
0–9  
-  
.  
_  
~

All other characters are percent encoded.

Example:

@ → %40  
! → %21

## Database Selection

Production runtime:

database = PROD_DB

Test runtime:

database = TEST_DB

## Credential Selection

Application runtime:

APP_DB_USER / APP_DB_PASSWORD

Migration operations:

NOMMIE_OWNER_USER / NOMMIE_OWNER_PASSWORD
