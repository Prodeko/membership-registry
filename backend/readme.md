# membership-registry

- Asenna sqlx-cli `cargo install sqlx-cli`

- Tyhjän migraation saa luotua komennolla `sqlx migrate add <kuvaus_ilman_välilyöntejä> -r`
- Aja migraatiot: `sqlx migrate run`
- Peruuta migraatio: `sqlx migrate revert`
- Luo SQL-kyselyistä tyypit CI:tä varten `cargo sqlx prepare -- --release --all-targets --all-features`
- Buildaa tailwind: `just tailwind`
- Käynnistä automaattinen tailwindin buildaus: `just watch tailwind`
- Aja prettier: `just prettier`
