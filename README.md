# Membership registry

## Prerequisites

- [Docker](https://docs.docker.com/get-docker/)
- [Rust](https://www.rust-lang.org/tools/install)
- [Node.js](https://nodejs.org/) and [pnpm](https://pnpm.io/installation)
- Python 3.12+ (for the Keycloak setup script)
- [sqlx-cli](https://crates.io/crates/sqlx-cli): `cargo install sqlx-cli`
- [Stripe CLI](https://docs.stripe.com/stripe-cli): `brew install stripe/stripe-cli/stripe` (or see docs for other OS)

## Quick start

```bash
# 1. Start infrastructure (Postgres on :5433, Keycloak on :8180, Mailpit on :8025)
docker compose up -d

# 2. Copy env templates
cp backend/.env.template backend/.env
cp frontend/.env.template frontend/.env
cp keycloak/.env.template keycloak/.env

# 3. Configure the Keycloak realm (idempotent — safe to re-run)
cd keycloak
python -m venv venv && source venv/bin/activate
pip install -e .
python setup.py
cd ..

# 4. Install frontend deps, run DB migrations
cd frontend && pnpm install && cd ..
cd backend && sqlx migrate run && cd ..

# 5. Start backend (terminal 1)
cd backend && cargo run

# 6. Start frontend (terminal 2)
cd frontend && pnpm run dev
```

App: http://127.0.0.1:5173 · Mailpit UI: http://localhost:8025 · Keycloak admin: http://localhost:8180 (admin / admin)

See [keycloak/README.md](keycloak/README.md) for more detail on the realm setup.

## Authentication (Keycloak)

Auth is handled by a local Keycloak instance configured by `keycloak/setup.py`. The script creates the `membership-registry` realm, two clients (a public auth client and an admin M2M client), and test users.

The backend needs these Keycloak variables in `backend/.env` (defaults in the template match what `setup.py` creates):

- `KEYCLOAK_URL`, `KEYCLOAK_REALM`
- `KEYCLOAK_CLIENT_ID` / `KEYCLOAK_CLIENT_SECRET` — OAuth2 client for login
- `KEYCLOAK_ADMIN_CLIENT_ID` / `KEYCLOAK_ADMIN_CLIENT_SECRET` — M2M client for user/role CRUD
- `OAUTH_REDIRECT_URL` — frontend callback (`http://127.0.0.1:5173/auth/callback`)

Test users created by `setup.py` (passwords in `keycloak/config/users.py`):

- `cto@prodeko.org` — admin role
- `user@prodeko.org` — regular user

Roles created in the database should also exist in Keycloak as realm roles. At minimum, an `admin` role must exist (created by `setup.py`).

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

To access the admin panel, your user needs the `admin` realm role in Keycloak. The `cto@prodeko.org` test user already has it. To grant it to another user: Keycloak admin console (http://localhost:8180) → realm `membership-registry` → Users → select user → Role mapping → Assign `admin`.

## Email configuration

Two adapters implement `EmailPort`, selected by env vars in `backend/.env`:

- **SMTP (dev default)** — `SMTP_HOST`, `SMTP_PORT`, `SMTP_FROM_EMAIL`. Points at Mailpit (`localhost:1025`) by default. View captured mail at http://localhost:8025.
- **SendGrid (prod)** — `SENDGRID_API_KEY`, `SENDGRID_FROM_EMAIL`, optionally `SENDGRID_API_URL` (defaults to `https://api.sendgrid.com`). Leave `SMTP_HOST` empty to activate.

If both are unset the backend refuses to start. Keycloak itself also sends mail (password reset, verification) — that's configured separately via `SENDGRID_API_KEY` in `keycloak/.env`.

## Mailchimp configuration

`MarketingListPort` keeps a Mailchimp audience in sync with membership: membership roles are the application-targetable roles (the ones members apply for and buy each year). Whenever such a role is granted, extended, renewed, deleted or expires, the user is subscribed (if they hold a membership role on that day and aren't a contact yet) or archived (if they no longer do). Members who unsubscribed themselves are never re-added, and only active members can use the profile page's subscribe button. Creating an account no longer subscribes anyone. `MAILCHIMP_API_KEY` and `MAILCHIMP_LIST_ID` must be set together, or both empty — a half-configuration makes the backend refuse to start.

In `backend/.env`:

- `MAILCHIMP_API_KEY` — must include the datacenter suffix (e.g. `abc123...-us21`)
- `MAILCHIMP_LIST_ID` — the audience/list ID

Leave both blank in dev to skip marketing sync entirely.

## Google Groups configuration

`GroupMembershipPort` keeps Google Workspace groups (the `jasenet@…` member lists that prodeko.org's application approval used to fill) in sync with the registry. Each group has a rule: the roles that put a user on it, and optionally a language and an attribute value the user must also have. A user is added to the group while they hold one of the rule's roles that day and meet its other conditions, and removed otherwise. The group is re-checked whenever one of those roles is granted, extended, renewed, deleted or expires, when the member's language changes, and when the attribute is set or cleared — including on the application form. When a member's email changes, the old address is removed from every group and the new one added where they belong; deleting a member removes their address from every group. Failures are logged and never fail the change that triggered them. The three variables must be set together, or all left empty. A partial or malformed configuration makes the backend refuse to start.

In `backend/.env`:

- `GOOGLE_SERVICE_ACCOUNT_KEY` — the service account's JSON key, as the variable's value (not a path)
- `GOOGLE_DELEGATED_ADMIN` — the Workspace admin the service account impersonates
- `GOOGLE_GROUP_RULES` — a JSON array of rules. `group` and `roles` are required; `language` and `attribute` are optional.

```json
[
  {"group": "jasenet@prodeko.org", "roles": ["prodeko-full-member", "prodeko-external-member"]},
  {"group": "jasenet@raittiusseura.org", "roles": ["prodeko-full-member", "prodeko-external-member"],
   "attribute": {"name": "pora-membership", "value": "yes"}}
]
```

Every member is on the Prodeko list, and members who ticked the PoRa box on the application form are also on the PoRa list. Add `"language": "fi"` to a rule to limit it to Finnish-speaking members, as the old prodeko.org did for the PoRa list.

The service account needs domain-wide delegation for the scope `https://www.googleapis.com/auth/admin.directory.group.member` in each Workspace whose groups it manages.

Leave all three blank in dev to skip group sync entirely.

## Migrations

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
- Test accounts: `cto@prodeko.org` / test (admin), `user@prodeko.org` / test

Emails are mocked — check with `docker compose -f docker-compose.prod.yml logs backend | grep "MOCK EMAIL"`. Stripe webhook secret is a dummy value. Migrations run automatically on backend startup.

Reset all data: `docker compose -f docker-compose.prod.yml down -v`

## Architecture

```text
backend/   - Rust (Axum) API server — DDD + Ports & Adapters
frontend/  - React + TypeScript + Vite
e2e/       - Playwright end-to-end tests
keycloak/  - Realm config + setup script (Python)
docs/      - Architecture notes
```

Backend source is organized into three layers under `src/`:

- `domain/` — pure business types, state machines, newtypes (no IO)
- `application/` — ports (traits) and services (use-case orchestration)
- `infrastructure/` — adapters (Keycloak, SendGrid, Stripe, …), repositories (sqlx), HTTP (Axum)

See [docs/architecture.md](docs/architecture.md) for the full breakdown.

## Running checks

The top-level `Makefile` mirrors CI. Postgres must be running (`make db-up`).

```bash
make -j ci      # all checks in parallel (backend + frontend + e2e-lint + audit)
make backend    # sqlx migrate, fmt, clippy, test, sqlx prepare --check
make frontend   # install, tsc, lint, prettier, build
make e2e-lint   # prettier check on e2e
make audit      # cargo audit + pnpm audit
```

Individual commands if you prefer:

```bash
# Backend
cd backend && cargo test && cargo clippy -- -D warnings

# Frontend
cd frontend && pnpm exec tsc --noEmit && pnpm run lint && pnpm exec prettier --check .

# E2E (requires backend + frontend running)
cp e2e/.env.e2e.template e2e/.env.e2e   # fill in test user passwords
cd e2e && pnpm exec playwright test
```
