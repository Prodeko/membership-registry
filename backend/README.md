# membership-registry

- Asenna sqlx-cli `cargo install sqlx-cli`

- Tyhjän migraation saa luotua komennolla `sqlx migrate add <kuvaus_ilman_välilyöntejä> -r`
- Aja migraatiot: `sqlx migrate run`
- Peruuta migraatio: `sqlx migrate revert`
- Luo SQL-kyselyistä tyypit CI:tä varten `cargo sqlx prepare -- --release --all-targets --all-features`

## Devdatan luominen

- Luo .env tiedosto .env_template pohjalta
- Aja `make devdata`
- Kirjaudu sisään aukeavaan browser ikkunaan
- Dev datan pitäisi olla ajettuna sisään tietokantaan
- Sulje browser ikkuna ja terminal prosessi

## Tiedostorakenne

Lähdekoodin alakansiot ovat seuraavat: http, repositories ja services.

http kansio sisältää http rajapinnan määrittelyn, repositories tietokantakyselyt ja services kaiken logiikan. Tarkoitus on, että http kansion alla olevat tiedostot kutsuvat vain services kansion tiedostoja, jotka kutsuvat sitten repositories tiedostoja tai muite servicejä. Repositories kansion tiedostot kutsuvat vain tietokantaa.
