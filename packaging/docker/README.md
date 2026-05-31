# Running fh6-tel-serve with Docker

The repo root `Dockerfile` builds a self-contained image of the headless
telemetry server (dashboard + UDP ingest), with the frontend embedded. Built on
Debian bullseye (glibc 2.31) so it runs anywhere modern.

## Pull the published image (recommended)

Each release pushes an image to GHCR:

```bash
docker pull ghcr.io/thebanhammer/fh6-tel-serve:latest
# or a specific version:  ghcr.io/thebanhammer/fh6-tel-serve:1.2.0
```

(If the package is private, make it public under the repo's *Packages* settings,
or `docker login ghcr.io` first.)

## Build it yourself

```bash
# from the repo root (build context = repo root)
docker build -t fh6-tel-serve .
```

## Run

```bash
docker run -d --name fh6-tel \
  -p 8080:8080 \
  -p 20440:20440/udp \
  -v fh6-tel-data:/data \
  --restart unless-stopped \
  fh6-tel-serve --ip 0.0.0.0 --port 8080 --auth-token CHANGE_ME
```

- **`-p 8080:8080`** — the web dashboard (open `http://<server-ip>:8080`).
- **`-p 20440:20440/udp`** — Forza telemetry ingest. Point the game's *Data Out*
  (Car Dash format) at the **server's IP on UDP 20440**.
- **`-v fh6-tel-data:/data`** — persists `sessions.db` + `settings.json`
  (`XDG_DATA_HOME=/data` inside the image). Without it, sessions are lost when the
  container is removed.
- **`--auth-token CHANGE_ME`** — recommended on a server; requires the browser
  login gate. Omit to leave the dashboard open to anyone who can reach it.
- Args after the image name are passed straight to `fh6-tel-serve`
  (`--ip / --port / --auth-token / --udp-port`).

## Docker Compose

The repo root also ships a [`docker-compose.yml`](../../docker-compose.yml):

```bash
cp .env.example .env     # set FH6_EXTRA_ARGS=--auth-token <secret>, host ports
docker compose up -d --build
```

`.env` knobs:

| Variable | Default | Purpose |
|----------|---------|---------|
| `FH6_HTTP_PORT` | `8080` | Host port for the dashboard (container always listens on 8080). |
| `FH6_UDP_PORT` | `20440` | Host UDP port for telemetry (container always listens on 20440). |
| `FH6_EXTRA_ARGS` | _(empty)_ | Extra flags for `fh6-tel-serve`, e.g. `--auth-token <secret>`. |

Data persists in the named volume `fh6-tel-data` (mounted at `/data`); it survives
`docker compose down` and is removed only by `docker compose down -v`. The compose
service includes a healthcheck that probes `GET /`.

## Notes

- The server is the single owner of the telemetry UDP socket and the SQLite DB —
  run one instance per database/volume.
- To change the UDP port, pass `--udp-port <port>` AND publish that port
  (`-p <port>:<port>/udp`).
- Firewall: open TCP `8080` and UDP `20440` (or your chosen ports) on the host.
- Logs: `docker logs -f fh6-tel`.
