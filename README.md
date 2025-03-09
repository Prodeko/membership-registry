# membership-registry

## Bootstrapping the development environment

**Make sure to bootstrap the ory environment first**
For more information check out [auth/README.md](../README.md)

**Copy the .env.template-files**
```bash
cp backend/.env.template backend/.env
cp frontend/.env.template frontend/.env
```

**Run npm install**
```bash
cd frontend
npm install
```

**Run backend migrations**
You need to install cargo and sqlx. For more information check out [backend/README.md](./backend/README.md)
```bash
sqlx migrate run
```

**Optionally create test data**
```bash
cd backend
cargo run -- generate --amount [amount]
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