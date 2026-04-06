# Membership registry

## Prerequisites

- [Docker](https://docs.docker.com/get-docker/)
- [Rust](https://www.rust-lang.org/tools/install)
- [Node.js](https://nodejs.org/)
- [pnpm](https://pnpm.io/installation)
- [sqlx-cli](https://crates.io/crates/sqlx-cli): `cargo install sqlx-cli`
- [Stripe CLI](https://docs.stripe.com/stripe-cli): `brew install stripe/stripe-cli/stripe` (or see docs for other OS)

## Quick start

```bash
# Start the database
docker compose up -d

# Set up environment files (see sections below)
cp backend/.env.template backend/.env
cp frontend/.env.template frontend/.env

# Install frontend dependencies and run migrations
cd frontend && pnpm install
cd backend && sqlx migrate run

# Start backend
cd backend && cargo run

# Start frontend in another terminal
cd frontend && pnpm run dev
```

## Auth0 configuration

We use a shared Auth0 dev tenant. Ask a team member for the credentials.

The backend needs two sets of Auth0 credentials in `backend/.env`:

1. **Regular application** (for login/OAuth flow):
   - `AUTH0_DOMAIN` - the tenant domain (e.g. `your-tenant.eu.auth0.com`)
   - `AUTH0_CLIENT_ID` - from the Auth0 application
   - `AUTH0_CLIENT_SECRET` - from the Auth0 application
   - `AUTH0_AUDIENCE` - API identifier (optional)
   - `OAUTH_REDIRECT_URL` - set to `http://127.0.0.1:5173/auth/callback`

2. **Machine-to-Machine (M2M) application** (for managing roles via the Management API):
   - `AUTH0_MANAGEMENT_CLIENT_ID`
   - `AUTH0_MANAGEMENT_CLIENT_SECRET`

Roles created locally in the database should also be created in Auth0 (Dashboard > User Management > Roles). At minimum, an **admin** role must exist in Auth0.

## Stripe configuration

Each developer uses their own [Stripe sandbox account](https://dashboard.stripe.com/test/dashboard).

1. Create a Stripe account and switch to sandbox mode
2. Create a product and a payment link for testing membership roles
3. Install the Stripe CLI and log in:

   ```bash
   stripe login
   ```

4. Forward webhook events to your local backend:

   ```bash
   stripe listen --forward-to 127.0.0.1:8080/api/stripe/webhook
   ```

5. Copy the webhook signing secret (`whsec_...`) that `stripe listen` prints and set it as `STRIPE_ENDPOINT_SECRET` in `backend/.env`

## Admin access

After starting the app, log in through the UI. The first login automatically creates a `UserAuthProvider` record and redirects you to the signup form. Fill it out to create your `Member` record.

To access the admin panel, you need the **admin** role assigned to your user in Auth0. You can do this from the Auth0 Dashboard (User Management > Users > your user > Roles > Assign Roles > admin).

To seed additional test data (roles, members, etc.), connect to the database directly:

```bash
docker compose exec membership-postgresd psql -U membership -d membership
```

Example SQL for creating a test role:

```sql
INSERT INTO Role (name, description, color) VALUES ('test-role', 'A test role', '#aabbcc');
```

### Migrations

```bash
cd backend

# Run migrations
sqlx migrate run

# Revert last migration
sqlx migrate revert

# Create a new migration
sqlx migrate add <description> -r

# Prepare query metadata for CI
cargo sqlx prepare -- --release --all-targets --all-features
```

## Production-like local stack

Mirrors the Azure deployment: Caddy reverse proxy, production Keycloak build, containerized backend.

```bash
# Start everything
docker compose -f docker-compose.prod.yml up --build -d

# Configure Keycloak realm (first time or after wiping volumes)
cd keycloak && source venv/bin/activate
KEYCLOAK_URL=http://auth.localhost \
KC_AUTH_REDIRECT_URIS='["http://localhost/*"]' \
KC_AUTH_WEB_ORIGINS='["http://localhost"]' \
python setup.py
```

- App: http://localhost
- Keycloak admin: http://auth.localhost (admin / admin)
- Test accounts: cto@prodeko.org / test (admin), user@prodeko.org / test

Emails are mocked — check with `docker compose -f docker-compose.prod.yml logs backend | grep "MOCK EMAIL"`. Stripe webhook secret is a dummy value. Migrations run automatically on backend startup.

Reset all data: `docker compose -f docker-compose.prod.yml down -v`

## Architecture

```text
backend/   - Rust (Axum) API server
frontend/  - React + TypeScript + Vite
e2e/       - Playwright end-to-end tests
```

Backend source is organized into three layers:

- **http** - route handlers (calls services only)
- **services** - business logic (calls repositories or other services)
- **repositories** - database queries

## Running checks

### Backend

```bash
cd backend

cargo test
cargo clippy
```

### Frontend

```bash
cd frontend

pnpm exec tsc --noEmit
pnpm run lint
pnpm exec prettier --check .
```

### End-to-end tests

```bash
cp e2e/.env.e2e.template e2e/.env.e2e
cd e2e && pnpm exec playwright test
```
