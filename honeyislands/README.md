# Honey Islands Battle is Solved

Two players alternately place black stones on the 61-cell Honey Islands board. Each stone discards the solutions in which that cell is white, and the player who leaves exactly one of the 1,304,983 solutions wins.

- **Standard rule** (stones only on undetermined cells): the first player wins; 42 of the 61 first moves win.
- **Redundant-hint rule** (stones may also go on cells already determined black, without shrinking the solution set): the first player wins with every first move.

Paper: [index.html](index.html) (English / 日本語). Game and analysis tool: [play/](play/).

The puzzle and its solution set come from the author's Honey Islands real-time solver (ハニーアイランドリアルタイムソルバ).

## Layout

| Path | Contents |
|---|---|
| `index.html`, `paper.css`, `paper.js` | the paper |
| `play/` | game and analysis tool (WebAssembly engine in a Web Worker, solution data, opening books) |
| `rust/` | engine: solver, AI, opening-book tools (`hib`), game-length search (`src/dist.rs`), wasm interface |
| `verify/` | independent checker `hibverify`, written from the rules alone (std only, no symmetry, no shared tables) |
| `results/normal`, `results/pass` | positions up to depth 3, edges, depth-3 results, winning depth-4 replies, first-move values, verifier outputs |
| `results/<rule>/best-line.json` | the perfect-play lines of Section 5.1 (`hib pv2`) |
| `results/analysis/` | best-line raw output (`pv2-*.txt`), random-game size (`gamesize.txt`) |
| `results/positions/` | number of positions (Section 5.2): estimator walks and summary (`tools/analysis/count_positions.py`, `exact_big.py`) |
| `results/pp/` | games sampled from the perfect-play tree (`hib ppgames`; `normal.txt`, `pass.txt`, and `*-err2.txt` with an error at move 3), their statistics and solver times (Sections 5.3, 7.2), and paired solves of 102 depth-3 positions (`hib isocost`; `iso-jobs.txt`, `isocost.txt`, `stats-iso.txt`, Section 7.3) |
| `tools/` | data conversion and checks, wasm build, book export, analysis scripts |
| `test/` | Node tests (JS reference engine, wasm engine, opening book, analysis) and match scripts |

`results/<rule>/depth3-results.txt` lines are `key value move n ms nodes stones`: canonical determined-black mask (`mask:passes` for the redundant-hint rule), 1 if the player to move wins, a winning cell or -1, remaining solutions, solve time, search nodes, and the stones of a representative. 601 standard-rule keys appear twice (solved again after jobs were reassigned); their values always agree.

## Build and test

Rust (stable, with the `wasm32-unknown-unknown` target), Python 3 and Node.js 22 or newer. No external crates or npm packages.

```sh
export CARGO_TARGET_DIR="$PWD/rust/target"
cargo test  --release --offline --manifest-path rust/Cargo.toml
cargo build --release --offline --manifest-path rust/Cargo.toml
cargo test  --release --offline --manifest-path verify/Cargo.toml
tools/build-wasm.sh
npm test
python3 -m http.server 8000 --bind 127.0.0.1   # then open http://127.0.0.1:8000/
```

## Reproduce the checks

```sh
# independent re-solve of the pass-rule proof set (164 positions, about 5 minutes on one thread)
cargo build --release --offline --manifest-path verify/Cargo.toml
verify/target/release/hibverify solve pass results/pass/verify-proofset-jobs.txt /tmp/res-pass.txt 1
python3 tools/analysis/verify_compare.py results/pass /tmp/res-pass.txt

# the standard-rule proof set (201 positions; 71 were checked before the run was stopped)
verify/target/release/hibverify solve normal results/normal/verify-proofset-jobs.txt /tmp/res-normal.txt <threads>

# rebuild the opening books used by the AI
python3 tools/book_export.py results/normal play/data/book-normal.bin 5   # needs the full known-position file, see below
```

The opening books in `play/data/` also contain about 290,000 positions with at most five determined-black cells. These come from the positions shared between machines during the solve. The full shared file (3.5 million positions, 58 MB) is not included, so `book_export.py` can rebuild only the depth-0–4 part from this repository.
