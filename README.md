# polito-wa2-ticketing

## Running the project

All commands run from the repository root unless stated otherwise.

| Service | Port when run locally (no Docker) | Port on your computer with Docker Compose |
|---|---|---|
| gateway | 3000 | 8080 |
| catalogue | 3001 | not exposed: reachable only through the gateway |
| inventory-orders | 3002 | not exposed: reachable only through the gateway |
| payments | 3003 | not exposed: reachable only from inside Docker |
| ticketing | 3004 | not exposed: reachable only through the gateway |
| notifications | 3005 | not exposed: reachable only from inside Docker |
| frontend | 5173 | 5173 |
| postgres | – | 5433 |
| adminer | – | 8090 |
| keycloak | – | 8180 |
| rustfs | – | 9000 (API), 9001 (console) |

Inside Docker, every service listens on port 8080; the other containers reach it as `http://<service>:8080`.

### 1. Everything with Docker Compose

```bash
docker compose up -d --build     # start
docker compose ps                # status
docker compose logs -f <service> # logs
docker compose down              # stop (add -v to delete the data)
```

### 2. Infrastructure only

```bash
docker compose up -d postgres adminer keycloak rustfs
```

Then run the services you need [locally](#4-without-docker) or [as single containers](#3-a-single-service-as-a-container).

### 3. A single service as a container

```bash
docker compose up -d --build <service>            # the service and what it depends on
docker compose up -d --build --no-deps <service>  # only the service
```

`<service>` is one of `gateway`, `catalogue`, `inventory-orders`, `payments`, `notifications`, `ticketing`, `frontend`. Several services can be listed in the same command.

Examples:

```bash
docker compose up -d --build catalogue                   # catalogue and postgres
docker compose up -d --build inventory-orders payments   # both, with postgres
docker compose up -d --build --no-deps gateway           # only the gateway
```

The gateway depends on all the other services, so without `--no-deps` it starts them too.

```bash
docker compose logs -f <service>   # logs
docker compose restart <service>   # restart
docker compose stop <service>      # stop
```

### 4. Without Docker

Start the [infrastructure](#2-infrastructure-only) first: the services connect to PostgreSQL on `localhost:5433`.

Kotlin (`gateway`, `inventory-orders`, `payments`, `notifications`):

```bash
cd services/<service>
./gradlew bootRun
```

Rust (`catalogue`, `ticketing`):

```bash
cd services/<service>
cp .env.example .env     # first time only
cargo run
```

Frontend:

```bash
cd services/frontend
npm install              # first time only
npm run dev
```

### Checks

```bash
./gradlew build                                                  # Kotlin: build and tests
cargo fmt --check && cargo clippy -- -D warnings && cargo test   # Rust
npm run lint && npm run build                                    # frontend
```