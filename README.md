## Dev setup

```
npm install -g json-server
json-server --watch db.json
```

Starts a local dev server on port 3000 with endpoints to mimic the production API.

http://localhost:3000/work

Then download and put Stockfish binary in the root of the project at filename `stockfish`.

## Run

```
cargo run
```




./stockfish
position fen rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1
go depth 20
