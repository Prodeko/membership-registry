FROM rust:1.72

ENV NODE_MAJOR=20
RUN curl -fsSL0 https://deb.nodesource.com/gpgkey/nodesource-repo.gpg.key \
    | gpg --dearmor -o /etc/apt/keyrings/nodesource.gpg \
    echo "deb [signed-by=/etc/apt/keyrings/nodesource.gpg] https://deb.nodesource.com/node_$NODE_MAJOR.x nodistro main" | tee /etc/apt/sources.list.d/nodesource.list

RUN apt-get update && apt-get install -y \
    musl-tools \
    ca-certificates \
    curl \
    gnupg \
    nodejs \
    npm

RUN rustup target add x86_64-unknown-linux-musl
RUN rustup component add rustfmt
RUN USER=root cargo new --bin boilerplate
WORKDIR /boilerplate

# Workaround for issue with postgres vscode extension https://github.com/microsoft/vscode-postgresql/issues/77
RUN wget http://mirrors.kernel.org/ubuntu/pool/main/libf/libffi/libffi6_3.2.1-8_amd64.deb && \
    apt install ./libffi6_3.2.1-8_amd64.deb

RUN cargo install cargo-watch

# Install SQLx CLI for database migrations (see README)
RUN cargo install sqlx-cli --no-default-features --features native-tls,postgres

RUN cargo install just