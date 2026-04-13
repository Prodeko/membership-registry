.PHONY: ci help backend frontend e2e-lint audit docker \
        backend-audit frontend-audit e2e-audit \
        db-up db-down

# Mirror CI env (see docker-compose.yml: postgres on 5433)
export DATABASE_URL      ?= postgres://membership:secret@localhost:5433/membership?sslmode=disable
export TEST_DATABASE_URL ?= $(DATABASE_URL)

# Group each parallel job's output and skip "Entering directory" noise.
MAKEFLAGS += --output-sync=recurse --no-print-directory

# Runs $(2) silently. Prints one completion line on success; on failure prints
# a banner + captured log and exits non-zero.
# Usage: $(call quiet,label,shell command)
define quiet
@LOG=$$(mktemp); START=$$(date +%s); \
if ( $(2) ) > $$LOG 2>&1; then \
  printf '[ok]   %-16s (%ss)\n' '$(1)' $$(($$(date +%s)-START)); \
  rm -f $$LOG; \
else \
  RC=$$?; \
  printf '[fail] %-16s (%ss)\n' '$(1)' $$(($$(date +%s)-START)); \
  cat $$LOG; \
  rm -f $$LOG; \
  exit $$RC; \
fi
endef

help:
	@echo "Local CI checks. Run in parallel with: make -j ci"
	@echo ""
	@echo "  make -j ci      all CI checks (parallel, silent unless failure)"
	@echo "  make backend    sqlx migrate, fmt, clippy, test, sqlx prepare --check"
	@echo "  make frontend   install, tsc, lint, prettier, build"
	@echo "  make e2e-lint   install, prettier"
	@echo "  make audit      cargo audit + pnpm audit (frontend & e2e)"
	@echo "  make docker     docker build"
	@echo "  make db-up      start postgres (docker compose)"
	@echo "  make db-down    stop docker compose"
	@echo ""
	@echo "Requires: postgres running (make db-up), sqlx-cli, pnpm, docker."

ci: backend frontend e2e-lint audit

backend:
	$(call quiet,backend, \
	  cd backend && \
	  sqlx migrate run && \
	  cargo fmt -- --check && \
	  cargo clippy -- -D warnings && \
	  cargo test && \
	  cargo sqlx prepare --check)

frontend:
	$(call quiet,frontend, \
	  cd frontend && \
	  pnpm install --frozen-lockfile && \
	  pnpm exec tsc --noEmit && \
	  pnpm run lint && \
	  pnpm exec prettier --check . && \
	  pnpm run build)

e2e-lint:
	$(call quiet,e2e-lint, \
	  cd e2e && \
	  pnpm install --frozen-lockfile && \
	  pnpm exec prettier --check .)

audit: backend-audit frontend-audit e2e-audit

backend-audit:
	$(call quiet,backend-audit,cd backend && cargo audit)

frontend-audit:
	$(call quiet,frontend-audit,cd frontend && pnpm audit --audit-level high)

e2e-audit:
	$(call quiet,e2e-audit,cd e2e && pnpm audit --audit-level high)

docker:
	$(call quiet,docker,docker build -t membership-registry:local .)

db-up:
	docker compose up -d postgres

db-down:
	docker compose down
