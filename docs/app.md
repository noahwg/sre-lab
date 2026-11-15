# The application

A URL shortener written in Rust (`axum` + `tokio`). It lives in `app/` and builds to a ~10 MB arm64 image. Links are stored in Postgres, behind a `LinkStore` trait. An in-memory implementation is used when `DATABASE_URL` is unset (tests and quick local runs), in which case links disappear on restart.

## Endpoints

| Method | Path | Auth | Purpose |
|---|---|---|---|
| `POST` | `/api/links` | API key | Create a link. Body: `{"url", "custom_code"?, "expires_at"?}`. Returns `201` |
| `GET` | `/api/links/{code}` | API key | Details and click count |
| `DELETE` | `/api/links/{code}` | API key | Delete (`204`) |
| `GET` | `/{code}` | public | `302` redirect; `404` unknown, `410` expired |
| `GET` | `/healthz` | public | Liveness |
| `GET` | `/readyz` | public | Readiness (store reachable) |
| `GET` | `/metrics` | public | Prometheus metrics. **Don't route this through the public proxy** |

The API key goes in the `x-api-key` header.

## Behavior

- **302, not 301**, with `Cache-Control: no-store`, so clicks are counted and links can change.
- Codes are random base62, 7 characters, retried on collision. Custom codes are 3-32 characters of `A-Za-z0-9-_`; `api`, `healthz`, `readyz` and `metrics` are reserved.
- Targets must be `http`/`https`, at most 2048 characters, no embedded credentials, and must not point at the service's own host.
- If the database is unreachable the API answers `503` (not `404`), `/readyz` goes unready, and `/healthz` stays `200` so the container isn't restarted for a database blip. `/readyz` runs a query against the `links` table, so it also catches missing migrations or wrong grants.
- Clicks are counted by a background task, so a redirect never waits on a write. At most 32 updates are in flight; beyond that clicks are dropped and counted.
- Metrics: `http_requests_total{method,route,status}`, `http_request_duration_seconds`, `redirects_total{outcome}`, `links_created_total`, `store_errors_total{op}`, `link_hits_failed_total`, `link_hits_dropped_total`. Routes are labelled by template (`/{code}`), so cardinality stays bounded.

## Configuration

| Env var | Default | Notes |
|---|---|---|
| `API_KEY` | required | At least 16 characters |
| `DATABASE_URL` | unset | `postgres://user:pass@host:5432/db`. Unset means the in-memory store. The pool is lazy (max 5 connections), so the app starts even if Postgres is down |
| `BASE_URL` | `http://localhost:8080` | Used to build `short_url` |
| `BIND_ADDR` | `0.0.0.0:8080` | |
| `LOG_FORMAT` | text | `json` for structured logs |
| `RUST_LOG` | `info` | |

## Develop

```sh
make app-test     # unit + API tests, in a container (no local Rust needed)
make app-image    # arm64 image, shortener:dev

docker run --rm -p 8080:8080 -e API_KEY=change-me-please-123 shortener:dev   # in-memory store
```

The Postgres store tests (`tests/pg_store.rs`) run only when `DATABASE_URL` points at a database with the migrations applied; otherwise they return immediately. CI runs them. Locally:

```sh
docker run -d --rm --name pg -p 5432:5432 -e POSTGRES_PASSWORD=pw -e POSTGRES_DB=shortener postgres:17-alpine
export DATABASE_URL='postgres://postgres:pw@host.docker.internal:5432/shortener?sslmode=disable'
docker run --rm -e DATABASE_URL -v "$PWD/app":/src -w /src -e CARGO_TARGET_DIR=/tmp/target rust:1-slim-bookworm sh -c 'cargo run -q -- migrate && cargo test'
```

The runtime image is distroless (no shell or curl), so the container healthcheck is `/app/shortener healthcheck`, a subcommand that calls `/healthz` on itself.

## Not done yet

OTLP trace export, rate limiting, and an expiry cleanup job.

## Database migrations

Schema changes are SQL files in `app/migrations/` (`0001_create_links.sql`, ...). They are embedded into the binary at build time (`sqlx::migrate!`), so the image carries its own migrations, which matters because the runtime image has no shell or `psql`.

- `shortener migrate` applies the pending files using `DATABASE_URL` and exits. It prints `migrate: applied N new, M total`.
- sqlx records each applied file, with a checksum, in `_sqlx_migrations`, and takes an advisory lock, so it is safe to run on every deploy and safe to run concurrently. **Editing an already-applied file fails** (`was previously applied but has been modified`). Add a new numbered file instead.
- It must run as the **owner** role (`shortener_owner`). The app's own runtime role cannot create or alter tables.

On the host, `make migrate` runs it as a one-off container on the `db` network (see [ansible.md](ansible.md#role-app)). `make deploy` runs it too, before starting the app.
