# Web UI

Open the server address in a browser for the server-backed UI, or open
`/wasm-demo/` from a local HTTP server after [building the WASM module](../getting-started/installation.md#build-the-browser-only-demo).
The browser-only variant uses the same UI but generates and solves mazes locally.
The published documentation serves it at [demo/wasm-demo/](../demo/wasm-demo/).

## Make a maze

1. Select a shape in **Shape**.
2. Select a generator in **Algorithm**.
3. Set **Width** and **Height**. The range is 5 to 321.
4. Set **Seed**, or click the dice button to get a random seed.
5. Click **Generate maze**.

## Edit a maze

Select a tool (**Wall**, **Start** or **Goals**), or push its key. Then click
a cell or drag over cells. The **Goals** tool is additive: click multiple
cells to place multiple goals. The maze panel shows the current goal count.

| Key | Tool | Result |
|-----|------|--------|
| `w` | Wall | Toggles the cell between wall and open |
| `s` | Start | Moves the start to the cell |
| `g` | Goals | Adds goals at the selected cells |

Clicks go to the correct cell for all shapes. This includes the notch of a
dart, which is concave.

## Solve a maze

Click **Run solver** or push `Enter`. The server-backed UI sends the maze
to the server; the browser-only variant solves it in WebAssembly. Both show the
path and the search statistics.

| Option | Result |
|--------|--------|
| **Show goal flood-fill** | Colours each cell by the loop in which the search reached it |
| **Animate loops** | Shows the search loop by loop, then the path |

For a square maze, click **Download maze for Rust** to save the maze as
`maze-WxH.txt`. Run the Rust solve example with:

```bash
cargo run -q --release --example solve < maze-WxH.txt
```

The Rust example reads square mazes as text. Other tilings need graph edges,
so they are not offered for this text download.

## Statistics

The statistics panel shows loops, tokens, token passes, reached cells, path
length and time. Heap use is only available from the server.

## How the UI draws a maze

The UI draws one image for the full board. It does not draw each cell as
a separate shape. This keeps large boards fast.

1. The UI makes a raster. Each pixel of the raster holds the id of the cell
   that covers it.
2. To fill one row of pixels for a cell, the UI finds all points where the
   row crosses the cell outline. It sorts the points. It fills the pixels
   between the first and second point, the third and fourth point, and so
   on. This even-odd rule is correct for convex and concave cells.
3. The UI keeps the raster until the board or the canvas size changes.
4. To draw, the UI writes one colour for each pixel from the cell state.
5. To find the cell under the mouse, the UI reads the raster at the mouse
   position.

A 321×321 kite-and-dart board makes its raster in about 70 ms. Each later
redraw takes about 5 ms.
