## Run

1. Download and put Stockfish binary in the root of this project at filename `stockfish`.
2. Generate API token with `engine:read` and `engine:write` scopes.
3. Run:

    cargo run lip_token_here

## Development

Sample stockfish command to see output:

```
./stockfish
position fen rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1
go depth 20
```
