# fh6-tel-serve — headless server

`fh6-tel-serve` runs the telemetry ingest + dashboard without a desktop window.
Run EITHER the desktop app OR the server on a machine (one process owns the UDP
socket and the SQLite database).

## Usage

    fh6-tel-serve [--ip 127.0.0.1] [--port 8080] [--auth-token <token>] [--udp-port <port>]

- `--ip`         HTTP bind address. Default `127.0.0.1`. Use `0.0.0.0` for LAN access.
- `--port`       HTTP port. Default `8080`.
- `--auth-token` When set, browsers must log in (HttpOnly session cookie).
- `--udp-port`   Override the Forza UDP port (otherwise from settings.json; default 20440).

Binding a non-localhost address without a token logs an open-instance warning.

Configure Forza to send telemetry (Car Dash format) to the server's IP on the UDP
port. Then open `http://<server-ip>:<port>` in any browser on the network.

See `systemd/`, `launchd/`, and `windows/` for running as a service.
