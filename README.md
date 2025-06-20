# Membership registry

## Bootstrapping the development environment

**Make sure to bootstrap the ory environment first**
For more information check out [auth/README.md](../README.md)

**Run setup**
Run the setup script to install the dependencies, create the database, and run the migrations.
```bash
make up
```


**Start the dev servers**
```bash
# In backend/
cargo run
```
```bash
# In frontend/
npm run dev
```

**Run a proxy between backend and ory for debuging**
```bash
mitmweb --listen-port 8081
```
