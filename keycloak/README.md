# Keycloak setup

`setup.py` configures the Keycloak realm via the Admin REST API. It is idempotent — safe to re-run after code changes or when resetting dev data.

**DO NOT TRY TO RUN THIS AGAINST PRODUCTION KEYCLOAK. THAT CONFIGURATION SHOULD BE DONE THROUGH THE KEYCLOAK ADMIN PANEL**

## What it configures

- Realm `membership-registry` (login settings, token lifetimes, email sender)
- Realm roles (including `admin`)
- Auth client `membership-registry` (public OAuth2 client for the frontend/backend)
- M2M client `membership-registry-m2m` (service account with `realm-management` roles for user/role CRUD from the backend)
- Test users (`cto@prodeko.org`, `user@prodeko.org`) — skip with `KC_SKIP_TEST_USERS=true`
- Custom browser + registration flows and passkey support

Module layout:

```
setup.py              - entry point
admin.py              - thin wrapper around the Keycloak Admin REST API
config/
  realm.py            - realm-level settings
  roles.py            - realm roles
  clients.py          - auth + M2M clients
  users.py            - test users
  flows.py            - browser / registration / passkey flows
themes/membership/    - custom login theme (mounted into Keycloak)
realm-export.json     - exported realm used by `start-dev --import-realm`
```

## First-time dev setup

Keycloak must be running — `docker compose up -d` from the repo root starts it on <http://localhost:8180>.

```bash
cd keycloak
python -m venv venv
source venv/bin/activate
pip install -e .          # install runtime deps
pip install -e '.[dev]'   # optional: add ruff + mypy for linting
cp .env.template .env     # fill in SENDGRID_API_KEY for prod-like email, blank is fine for dev
python setup.py
```

The script waits up to 60s for Keycloak to become available, then applies the configuration.

## Overriding for the prod-like stack

`docker-compose.prod.yml` runs Keycloak behind Caddy at `http://auth.localhost`. Pass matching URLs when running `setup.py`:

```bash
KEYCLOAK_URL=http://auth.localhost \
KC_AUTH_REDIRECT_URIS='["http://localhost/*"]' \
KC_AUTH_WEB_ORIGINS='["http://localhost"]' \
python setup.py
```

## Environment variables

- `KEYCLOAK_URL` — Keycloak base URL (default `http://localhost:8180`)
- `KC_SKIP_TEST_USERS` — set to `true` to skip creating test users
- `KC_AUTH_REDIRECT_URIS`, `KC_AUTH_WEB_ORIGINS` — JSON arrays for the auth client
- `SENDGRID_API_KEY`, `SMTP_FROM`, `SMTP_FROM_NAME` — SendGrid SMTP for emails sent by Keycloak itself (password reset, verification). Leave blank in dev.

## Linting and type checking

```bash
source venv/bin/activate
ruff check .
mypy .
```

Dev dependencies are declared in `pyproject.toml` under `[project.optional-dependencies]`.
