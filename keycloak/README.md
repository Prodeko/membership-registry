# Keycloak Setup

`setup.py` configures the Keycloak realm via the Admin REST API. It is idempotent and safe to run multiple times.

```bash
cp .env.template .env   # fill in SENDGRID_API_KEY etc.
python setup.py
```

The script expects Keycloak to be running (via `docker compose up`) and will wait up to 60 seconds for it to become available.

## Linting and type checking

```bash
source venv/bin/activate
ruff check .
mypy .
```

Dev dependencies are listed in `pyproject.toml` under `[project.optional-dependencies]`.
