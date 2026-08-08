# Stage 1: Frontend build
FROM node:22-bookworm-slim AS frontend
RUN corepack enable
WORKDIR /frontend
COPY frontend/package.json frontend/pnpm-lock.yaml frontend/pnpm-workspace.yaml ./
RUN pnpm install --frozen-lockfile
COPY frontend/ .
ARG VITE_API_BASE_URL=/api
RUN pnpm run build

# Stage 2: Backend build
FROM rust:1.94-bookworm AS backend
WORKDIR /app
COPY backend/Cargo.toml backend/Cargo.lock ./
RUN mkdir src && echo 'fn main(){}' > src/main.rs && cargo build --release && rm -rf src
COPY backend/ .
RUN touch src/main.rs
ENV SQLX_OFFLINE=true
RUN cargo build --release

# Stage 3: Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=backend /app/target/release/membership-registry /usr/local/bin/
COPY --from=backend /app/migrations /app/migrations
COPY --from=frontend /frontend/dist /app/frontend/dist
WORKDIR /app
EXPOSE 8080
CMD ["membership-registry"]
