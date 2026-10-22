//! C ABI for the web worker (wasm32). Single-threaded: state lives in a
//! thread-local. Buffers are owned here; JS reads/writes them via pointers.

use crate::{list_counts, legal, Known, Mcts, Params, Sym, Universe, N};
use std::cell::RefCell;

struct State {
    u: Universe,
    m: Mcts,
    /// opening books: normal rule in `map`, pass rule in `pmap`
    book: Known,
    sym: Sym,
    has_book: [bool; 2],
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    static STONES: RefCell<[u8; 64]> = const { RefCell::new([0; 64]) };
    static OUT_U32: RefCell<[u32; 2 + N]> = const { RefCell::new([0; 2 + N]) };
    static OUT_F64: RefCell<[f64; 8]> = const { RefCell::new([0.0; 8]) };
    static OUT_EVAL: RefCell<[i8; 2 * N]> = const { RefCell::new([0; 2 * N]) };
}

#[no_mangle]
pub extern "C" fn hib_alloc(len: usize) -> *mut u8 {
    let mut v = Vec::<u8>::with_capacity(len);
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

/// Takes ownership of `len` bytes at `ptr` (solutions.bin). Returns the count.
#[no_mangle]
pub extern "C" fn hib_init(ptr: *mut u8, len: usize, seed: u32) -> u32 {
    let bytes = unsafe { Vec::from_raw_parts(ptr, len, len) };
    let u = Universe::from_le_bytes(&bytes);
    drop(bytes);
    let n = u.sols.len() as u32;
    let m = Mcts::new(Params::default(), seed as u64, 22);
    STATE.with(|s| *s.borrow_mut() = Some(State { u, m, book: Known::new(0), sym: Sym::new(), has_book: [false; 2] }));
    n
}

#[no_mangle]
pub extern "C" fn hib_stones() -> *mut u8 {
    STONES.with(|s| s.borrow_mut().as_mut_ptr())
}

fn stones(k: usize) -> Vec<u8> {
    STONES.with(|s| s.borrow()[..k.min(64)].to_vec())
}

/// out[0] = remaining solutions, out[1] = number of legal moves,
/// out[2 + c] = solutions with cell c black.
#[no_mangle]
pub extern "C" fn hib_analyze(k: usize) -> *const u32 {
    let st = stones(k);
    STATE.with(|s| {
        let s = s.borrow();
        let s = s.as_ref().expect("hib_init first");
        let set = s.u.play(&st);
        let cnt = match &set {
            crate::Set::List(v) => list_counts(v),
            _ => s.u.counts(&set),
        };
        OUT_U32.with(|o| {
            let mut o = o.borrow_mut();
            o[0] = set.n() as u32;
            o[1] = legal(&cnt, set.n()).len() as u32;
            o[2..].copy_from_slice(&cnt);
            o.as_ptr()
        })
    })
}

/// Takes ownership of `len` bytes at `ptr` (book-<rule>.bin). Returns the record count.
#[no_mangle]
pub extern "C" fn hib_book(ptr: *mut u8, len: usize, rule: u32) -> u32 {
    let bytes = unsafe { Vec::from_raw_parts(ptr, len, len) };
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        let s = s.as_mut().expect("hib_init first");
        s.has_book[(rule == 1) as usize] = true;
        s.book.load_bin(&bytes, rule == 1) as u32
    })
}

/// out = [move, proven, value, iterations, playouts, ms, from_book]
/// rule: 0 = normal, 1 = stones may also go on cells determined black (passes)
#[no_mangle]
pub extern "C" fn hib_think(k: usize, ms: f64, seed: u32, rule: u32) -> *const f64 {
    let st = stones(k);
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        let s = s.as_mut().expect("hib_init first");
        if s.has_book[(rule == 1) as usize] {
            let t = crate::now_ms();
            if let Some(c) = crate::book_choose(&s.u, &st, rule == 1, &s.book, &s.sym, seed as u64) {
                return OUT_F64.with(|o| {
                    let mut o = o.borrow_mut();
                    *o = [c as f64, 1.0, 1.0, 0.0, 0.0, crate::now_ms() - t, 1.0, 0.0];
                    o.as_ptr()
                });
            }
        }
        if rule == 1 {
            let t = crate::now_ms();
            let (mv, proven, value) = crate::pass_choose(&s.u, &st, ms, seed as u64, &mut s.m.solver);
            return OUT_F64.with(|o| {
                let mut o = o.borrow_mut();
                *o = [mv as f64, proven as u8 as f64, value, 0.0, 0.0, crate::now_ms() - t, 0.0, 0.0];
                o.as_ptr()
            });
        }
        s.m.reseed(seed as u64);
        let r = s.m.search(&s.u, &st, ms);
        OUT_F64.with(|o| {
            let mut o = o.borrow_mut();
            *o = [r.mv as f64, r.proven as u8 as f64, r.value, r.iters as f64, r.playouts as f64, r.ms, 0.0, 0.0];
            o.as_ptr()
        })
    })
}

/// Value of every move for the player to move (analysis view).
/// out[c] = -1 not playable, 0 the move loses, 1 the move wins, 2 unknown (time ran out);
/// out[N + c] = source: 0 none, 1 opening book, 2 solved now.
/// Pass rule: every free determined-black cell gets the value of passing.
/// Moves are looked up in the book first, then solved smallest child first within `ms`.
#[no_mangle]
pub extern "C" fn hib_eval(k: usize, ms: f64, rule: u32) -> *const i8 {
    let st = stones(k);
    let pass = rule == 1;
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        let s = s.as_mut().expect("hib_init first");
        let mut out = [-1i8; 2 * N];
        out[N..].fill(0);
        let list = s.u.to_list(&s.u.play(&st));
        let n = list.len();
        if n > 1 {
            let full = (1u64 << N) - 1;
            let dm = list.iter().fold(full, |a, &x| a & x);
            let passes = dm.count_ones() as i32 - st.len() as i32;
            let stm = st.iter().fold(0u64, |m, &c| m | 1u64 << c);
            let cnt = list_counts(&list);
            // options: (cell, child, passes left in the child, is_pass)
            let mut opts: Vec<(usize, Vec<u64>, i32, bool)> = Vec::new();
            if pass && passes > 0 {
                opts.push((N, list.clone(), passes - 1, true));
            }
            for c in legal(&cnt, n) {
                let c = c as usize;
                let kid: Vec<u64> = list.iter().copied().filter(|x| (x >> c) & 1 == 1).collect();
                let kd = kid.iter().fold(full, |a, &x| a & x);
                opts.push((c, kid, passes + kd.count_ones() as i32 - dm.count_ones() as i32 - 1, false));
            }
            opts.sort_by_key(|o| o.1.len());
            let t0 = crate::now_ms();
            let mut vals: Vec<(i8, i8)> = Vec::with_capacity(opts.len());
            for (_, kid, p2, _) in &opts {
                let kn = kid.len();
                if kn == 1 {
                    vals.push((1, 2));
                    continue;
                }
                let kd = kid.iter().fold(full, |a, &x| a & x);
                let key = s.sym.canon(kd);
                let hit = if !s.has_book[pass as usize] {
                    None
                } else if pass {
                    s.book.pmap.read().unwrap().get(&(key, *p2)).copied()
                } else {
                    s.book.map.read().unwrap().get(&key).copied()
                };
                if let Some(v) = hit {
                    vals.push((!v as i8, 1));
                    continue;
                }
                let left = t0 + ms - crate::now_ms();
                if left <= 0.0 {
                    vals.push((2, 0));
                    continue;
                }
                let sv = &mut s.m.solver;
                sv.deadline = t0 + ms;
                sv.node_limit = u64::MAX;
                let r = if pass { sv.win_pass(kid, *p2) } else { sv.win(kid) };
                vals.push(match r {
                    Ok(v) => (!v as i8, 2),
                    Err(_) => (2, 0),
                });
            }
            for ((c, _, _, is_pass), (v, src)) in opts.iter().zip(vals) {
                if *is_pass {
                    for d in 0..N {
                        if dm >> d & 1 == 1 && stm >> d & 1 == 0 {
                            out[d] = v;
                            out[N + d] = src;
                        }
                    }
                } else {
                    out[*c] = v;
                    out[N + *c] = src;
                }
            }
        }
        OUT_EVAL.with(|o| {
            let mut o = o.borrow_mut();
            *o = out;
            o.as_ptr()
        })
    })
}
