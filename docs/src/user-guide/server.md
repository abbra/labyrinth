# Web Server

The `labyrinth-server` crate is a small web application. It does these
tasks:

- It makes boards of five shapes.
- It generates mazes on the boards.
- It solves mazes with the model.
- It serves a browser UI.

## Start the server

```bash
cargo run --release -p labyrinth-server
```

The server writes its address when it is ready:

```text
listening on http://127.0.0.1:3000
```

## Configuration

| Variable | Default | Meaning |
|----------|---------|---------|
| `LABYRINTH_ADDR` | `127.0.0.1:3000` | The address and port to listen on |

The server has no other settings.

## Limits

| Limit | Value | Reason |
|-------|-------|--------|
| Smallest side | 5 | The square generators need space for separate start and goal rooms |
| Largest side | 321 | Bounds board size and per-request resource use |

The server rejects a request with a side out of these limits. It sends
status `400`.

## Execution

The solver itself is sequential. The server runs each solve on a blocking
thread, with at most two solves in flight, so a solve does not block the
async runtime.

## More information

- [HTTP API](http-api.md) gives the routes and the JSON fields.
- [Web UI](web-ui.md) tells how to use the browser UI.
- [Tilings and Generators](tilings.md) describes the board shapes.
