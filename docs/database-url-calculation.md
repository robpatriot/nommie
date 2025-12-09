# Database URL Construction

## Environment Variables Used

### Required Variables (All Scenarios)

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `POSTGRES_HOST` | No | `"localhost"` | Database hostname |
| `POSTGRES_PORT` | No | `"5432"` | Database port |
| `PROD_DB` | Yes (Prod) | - | Production database name |
| `TEST_DB` | Yes (Test) | - | Test database name (must end with `_test`) |
| `APP_DB_USER` | Yes (App role) | - | Application database username |
| `APP_DB_PASSWORD` | Yes (App role) | - | Application database password |
| `NOMMIE_OWNER_USER` | Yes (Owner role) | - | Owner database username (for migrations) |
| `NOMMIE_OWNER_PASSWORD` | Yes (Owner role) | - | Owner database password (for migrations) |

### SSL/TLS Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `POSTGRES_SSL_MODE` | No | `"verify-full"` | SSL mode: `disable`, `require`, `verify-ca`, `verify-full` |
| `POSTGRES_SSL_ROOT_CERT` | Conditional* | - | Absolute path to CA certificate file |

*Required when `POSTGRES_SSL_MODE` is NOT `"disable"`

## URL Construction Formula

### Base URL Format
```
postgresql://{encoded_username}:{encoded_password}@{host}:{port}/{database_name}
```

### With SSL (when POSTGRES_SSL_MODE ≠ "disable")
```
postgresql://{encoded_username}:{encoded_password}@{host}:{port}/{database_name}?sslmode={ssl_mode}&sslrootcert={root_cert_path}
```

### Without SSL (when POSTGRES_SSL_MODE = "disable")
```
postgresql://{encoded_username}:{encoded_password}@{host}:{port}/{database_name}
```

## Example Calculations

### Example 1: Production with SSL (Default - App Role)

**Environment Variables:**
```bash
POSTGRES_HOST=db.example.com
POSTGRES_PORT=5432
PROD_DB=nommie
APP_DB_USER=nommie_app
APP_DB_PASSWORD=secure-password-123
POSTGRES_SSL_MODE=verify-full  # (default, can be omitted)
POSTGRES_SSL_ROOT_CERT=/etc/ssl/certs/nommie-ca.crt
```

**Calculation:**
1. Username encoding: `nommie_app` → `nommie_app` (no special chars)
2. Password encoding: `secure-password-123` → `secure-password-123` (no special chars)
3. Base URL: `postgresql://nommie_app:secure-password-123@db.example.com:5432/nommie`
4. SSL mode check: `verify-full` ≠ `disable` → append SSL params
5. Final URL: `postgresql://nommie_app:secure-password-123@db.example.com:5432/nommie?sslmode=verify-full&sslrootcert=/etc/ssl/certs/nommie-ca.crt`

### Example 2: Production with SSL (Owner Role for Migrations)

**Environment Variables:**
```bash
POSTGRES_HOST=db.example.com
POSTGRES_PORT=5432
PROD_DB=nommie
NOMMIE_OWNER_USER=nommie_owner
NOMMIE_OWNER_PASSWORD=owner-password-456
POSTGRES_SSL_MODE=verify-full
POSTGRES_SSL_ROOT_CERT=/etc/ssl/certs/nommie-ca.crt
```

**Final URL:**
```
postgresql://nommie_owner:owner-password-456@db.example.com:5432/nommie?sslmode=verify-full&sslrootcert=/etc/ssl/certs/nommie-ca.crt
```

### Example 3: Test Environment with SSL

**Environment Variables:**
```bash
POSTGRES_HOST=localhost
POSTGRES_PORT=5432
TEST_DB=nommie_test
APP_DB_USER=nommie_app
APP_DB_PASSWORD=dev-password
POSTGRES_SSL_MODE=verify-full
POSTGRES_SSL_ROOT_CERT=/path/to/ca.crt
```

**Final URL:**
```
postgresql://nommie_app:dev-password@localhost:5432/nommie_test?sslmode=verify-full&sslrootcert=/path/to/ca.crt
```

### Example 4: Local Development without SSL

**Environment Variables:**
```bash
POSTGRES_HOST=localhost
POSTGRES_PORT=5432
PROD_DB=nommie
APP_DB_USER=nommie_app
APP_DB_PASSWORD=dev-password
POSTGRES_SSL_MODE=disable
# POSTGRES_SSL_ROOT_CERT not needed when SSL is disabled
```

**Final URL:**
```
postgresql://nommie_app:dev-password@localhost:5432/nommie
```

### Example 5: Username/Password with Special Characters

**Environment Variables:**
```bash
POSTGRES_HOST=localhost
POSTGRES_PORT=5432
PROD_DB=nommie
APP_DB_USER=user@domain
APP_DB_PASSWORD=p@ssw0rd!123
POSTGRES_SSL_MODE=verify-full
POSTGRES_SSL_ROOT_CERT=/etc/ssl/certs/ca.crt
```

**Calculation:**
1. Username encoding: `user@domain` → `user%40domain` (`@` becomes `%40`)
2. Password encoding: `p@ssw0rd!123` → `p%40ssw0rd%21123` (`@` → `%40`, `!` → `%21`)
3. Base URL: `postgresql://user%40domain:p%40ssw0rd%21123@localhost:5432/nommie`
4. Final URL: `postgresql://user%40domain:p%40ssw0rd%21123@localhost:5432/nommie?sslmode=verify-full&sslrootcert=/etc/ssl/certs/ca.crt`

## Username/Password Encoding

The code uses `percent_encoding` with a custom `USERINFO_ENCODE_SET` that:
- **Allows** (unreserved): `a-z`, `A-Z`, `0-9`, `-`, `.`, `_`, `~`
- **Encodes** everything else (e.g., `@` → `%40`, `!` → `%21`, `#` → `%23`, etc.)

This follows RFC 3986 userinfo encoding rules.

## Code Location

The URL construction happens in:
- **File**: `apps/backend/src/config/db.rs`
- **Function**: `make_conn_spec()` (lines 278-351)
- **Encoding function**: `encode_userinfo()` (lines 203-205)

## Summary Table

| Scenario | Env | Owner | SSL | Required Vars |
|----------|-----|-------|-----|---------------|
| Production App | `Prod` | `App` | Yes | `PROD_DB`, `APP_DB_USER`, `APP_DB_PASSWORD`, `POSTGRES_SSL_ROOT_CERT` |
| Production App | `Prod` | `App` | No | `PROD_DB`, `APP_DB_USER`, `APP_DB_PASSWORD`, `POSTGRES_SSL_MODE=disable` |
| Production Owner | `Prod` | `Owner` | Yes | `PROD_DB`, `NOMMIE_OWNER_USER`, `NOMMIE_OWNER_PASSWORD`, `POSTGRES_SSL_ROOT_CERT` |
| Test App | `Test` | `App` | Yes | `TEST_DB`, `APP_DB_USER`, `APP_DB_PASSWORD`, `POSTGRES_SSL_ROOT_CERT` |
| Test App | `Test` | `App` | No | `TEST_DB`, `APP_DB_USER`, `APP_DB_PASSWORD`, `POSTGRES_SSL_MODE=disable` |

**Note**: `POSTGRES_HOST` and `POSTGRES_PORT` have defaults, so they're optional but typically set.

