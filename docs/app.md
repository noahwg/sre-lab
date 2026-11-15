# The application

A URL shortener written in Rust (`axum` + `tokio`). It lives in `app/` and builds to a ~10 MB arm64 image. **Storage is in-memory for now**, behind a `LinkStore` trait, so links disappear on restart. A Postgres implementation replaces it without touching the handlers.

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
- Metrics: `http_requests_total{method,route,status}`, `http_request_duration_seconds`, `redirects_total{outcome}`, `links_created_total`. Routes are labelled by template (`/{code}`), so cardinality stays bounded.

## Configuration

| Env var | Default | Notes |
|---|---|---|
| `API_KEY` | required | At least 16 characters |
| `BASE_URL` | `http://localhost:8080` | Used to build `short_url` |
| `BIND_ADDR` | `0.0.0.0:8080` | |
| `LOG_FORMAT` | text | `json` for structured logs |
| `RUST_LOG` | `info` | |

## Develop

```sh
make app-test     # unit + API tests, in a container (no local Rust needed)
make app-image    # arm64 image, shortener:dev

docker run --rm -p 8080:8080 -e API_KEY=change-me-please-123 shortener:dev
```

The runtime image is distroless (no shell or curl), so the container healthcheck is `/app/shortener healthcheck`, a subcommand that calls `/healthz` on itself.

## Not done yet

Postgres store, OTLP trace export, rate limiting, and the Ansible deploy role plus ECR.

## Database migrations

Schema changes are SQL files in `app/migrations/` (`0001_create_links.sql`, ...). They are embedded into the binary at build time (`sqlx::migrate!`), so the image carries its own migrations, which matters because the runtime image has no shell or `psql`.

- `shortener migrate` applies the pending files using `DATABASE_URL` and exits. It prints `migrate: applied N new, M total`.
- sqlx records each applied file, with a checksum, in `_sqlx_migrations`, and takes an advisory lock, so it is safe to run on every deploy and safe to run concurrently. **Editing an already-applied file fails** (`was previously applied but has been modified`). Add a new numbered file instead.
- It must run as the **owner** role (`shortener_owner`). The app's own runtime role cannot create or alter tables.

On the host, `make migrate` runs it as a one-off container on the `db` network (see [ansible.md](ansible.md#role-app)). `make deploy` runs it too, before starting the app.
