//! Native tools: `hib bench` (solver speed), `hib match <A> <B> <games> <ms> [offset]`
//! where players are `mcts` (default params, env overrides) or `flat`.
use hib_engine::*;

fn load() -> Universe {
    let path = std::env::var("HIB_DATA").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../analyze/data/solutions.bin").to_string());
    Universe::from_le_bytes(&std::fs::read(path).expect("solutions.bin"))
}

fn env<T: std::str::FromStr>(k: &str, d: T) -> T {
    std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d)
}

fn params() -> Params {
    let d = Params::default();
    Params {
        c: env("C", d.c),
        smart_playout: env("SMART", 1u8) == 1,
        handoff: env("HANDOFF", d.handoff),
        proof_limit: env("PROOF_LIMIT", d.proof_limit),
        proof_share: env("PROOF_SHARE", d.proof_share),
        exact_limit: env("EXACT_LIMIT", d.exact_limit),
        solver_nodes: env("SOLVER_NODES", d.solver_nodes),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let t = now_ms();
    let u = load();
    eprintln!("universe {} solutions, {:.0} ms", u.sols.len(), now_ms() - t);
    match args.get(1).map(|s| s.as_str()) {
        Some("line") => {
            // hib line <stones|-> : best line from a position (RULE=pass for the pass rule).
            // The winner plays the fastest win, the loser the longest resistance.
            // One row per ply: "ply n passes mover_wins plies_to_end move is_pass ties nodes ms".
            let pass = std::env::var("RULE").is_ok_and(|r| r == "pass");
            let mut st: Vec<u8> = if args.get(2).is_none_or(|a| a == "-") { Vec::new() } else { args[2].split(',').map(|x| x.parse().unwrap()).collect() };
            let mut ds = hib_engine::dist::Dist::new(pass);
            fn dm(v: &[u64]) -> u64 { v.iter().fold(!0u64, |a, &x| a & x) & ((1u64 << N) - 1) }
            let t0 = now_ms();
            loop {
                let list = u.to_list(&u.play(&st));
                let n = list.len();
                if n <= 1 { break; }
                let d0 = dm(&list);
                let p = d0.count_ones() as i32 - st.len() as i32;
                let (win, len) = ds.length(&list, p).ok().unwrap();
                let cnt = list_counts(&list);
                // options: (cell, is_pass, child, passes left in the child)
                let mut opts: Vec<(u8, bool, Vec<u64>, i32)> = Vec::new();
                if pass && p > 0 {
                    let stm = st.iter().fold(0u64, |m, &c| m | 1u64 << c);
                    let c = (0..N as u8).find(|&c| d0 >> c & 1 == 1 && stm >> c & 1 == 0).unwrap();
                    opts.push((c, true, list.clone(), p - 1));
                }
                let mut lg = legal(&cnt, n);
                lg.sort_unstable_by_key(|&c| cnt[c as usize]);
                for c in lg {
                    let kid: Vec<u64> = list.iter().copied().filter(|x| (x >> c) & 1 == 1).collect();
                    let p2 = p + dm(&kid).count_ones() as i32 - d0.count_ones() as i32 - 1;
                    opts.push((c, false, kid, p2));
                }
                let (mut pick, mut ties) = (None, 0);
                if win {
                    for (i, o) in opts.iter().enumerate() {
                        if ds.loses_within(&o.2, o.3, len - 1).ok().unwrap() {
                            if pick.is_none() { pick = Some(i); }
                            ties += 1;
                        }
                    }
                } else {
                    let mut best = 0;
                    for (i, o) in opts.iter().enumerate() {
                        let (_, l) = ds.length(&o.2, o.3).ok().unwrap();
                        if l + 1 > best { best = l + 1; pick = Some(i); ties = 0; }
                        if l + 1 == best { ties += 1; }
                    }
                    assert_eq!(best, len);
                }
                let o = &opts[pick.unwrap()];
                println!("{} {n} {p} {} {len} {} {} {ties} {} {:.0}", st.len(), win as u8, o.0, o.1 as u8, ds.nodes, now_ms() - t0);
                st.push(o.0);
            }
            println!("line {}", st.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(","));
        }
        Some("pv2") => {
            // hib pv2 <known-all.txt> <book.bin> [thresh] : a best line in two stages (RULE=pass
            // for the pass rule). Above `thresh` solutions (default 3000) the winner plays the
            // winning move leaving the fewest solutions and the loser the move keeping the most
            // (values from the book / shared results, else solved). From `thresh` down, exact
            // game length: the winner finishes fastest, the loser resists longest.
            // Rows: "ply n passes mover_wins stage move is_pass plies_left(stage 2) ms"
            use std::sync::Arc;
            let pass = std::env::var("RULE").is_ok_and(|r| r == "pass");
            let thresh: usize = args.get(4).map(|s| s.parse().unwrap()).unwrap_or(3000);
            let known = Arc::new(Known::new(3000));
            known.load_text(&std::fs::read_to_string(&args[2]).unwrap());
            known.load_bin(&std::fs::read(&args[3]).unwrap(), pass);
            eprintln!("known {} / {}", known.map.read().unwrap().len(), known.pmap.read().unwrap().len());
            let mut sv = Solver::new(24).with_symmetry();
            sv.known = Some(known.clone());
            let sym = Sym::new();
            let mut ds = hib_engine::dist::Dist::new(pass);
            fn dm(v: &[u64]) -> u64 { v.iter().fold(!0u64, |a, &x| a & x) & ((1u64 << N) - 1) }
            let t0 = now_ms();
            let value = |list: &[u64], p: i32, sv: &mut Solver| -> bool {
                if list.len() <= 1 { return false; }
                let key = sym.canon(dm(list));
                let hit = if pass { known.pmap.read().unwrap().get(&(key, p)).copied() } else { known.map.read().unwrap().get(&key).copied() };
                if let Some(v) = hit { return v; }
                let r = if pass { sv.win_pass(list, p) } else { sv.win(list) };
                r.ok().unwrap()
            };
            let mut st: Vec<u8> = Vec::new();
            let mut list = u.to_list(&u.root());
            let mut p = 0i32;
            let mut ply = 0;
            while list.len() > 1 {
                let n = list.len();
                let d0 = dm(&list);
                let stm = st.iter().fold(0u64, |m, &c| m | 1u64 << c);
                // options: (cell, is_pass, child, passes left in the child)
                let mut opts: Vec<(u8, bool, Vec<u64>, i32)> = Vec::new();
                if pass && p > 0 {
                    let c = (0..N as u8).find(|&c| d0 >> c & 1 == 1 && stm >> c & 1 == 0).unwrap();
                    opts.push((c, true, list.clone(), p - 1));
                }
                let cnt = list_counts(&list);
                let mut lg = legal(&cnt, n);
                lg.sort_unstable_by_key(|&c| cnt[c as usize]);
                for c in lg {
                    let kid: Vec<u64> = list.iter().copied().filter(|x| (x >> c) & 1 == 1).collect();
                    let p2 = p + dm(&kid).count_ones() as i32 - d0.count_ones() as i32 - 1;
                    opts.push((c, false, kid, p2));
                }
                let (pick, win, stage, left);
                if n > thresh {
                    stage = 1;
                    left = 0;
                    // winning move with the fewest solutions (regular moves are sorted; a pass keeps n)
                    let mut w = None;
                    // a position recorded as lost needs no search: every option loses
                    let key = sym.canon(d0);
                    let lost = if pass { known.pmap.read().unwrap().get(&(key, p)) == Some(&false) } else { known.map.read().unwrap().get(&key) == Some(&false) };
                    let order: Vec<usize> = (0..opts.len()).filter(|&i| !opts[i].1).chain((0..opts.len()).filter(|&i| opts[i].1)).collect();
                    for &i in order.iter().filter(|_| !lost) {
                        if opts[i].2.len() == 1 || !value(&opts[i].2, opts[i].3, &mut sv) { w = Some(i); break; }
                    }
                    win = w.is_some();
                    pick = w.unwrap_or_else(|| (0..opts.len()).max_by_key(|&i| (opts[i].2.len(), opts[i].1 as u8)).unwrap());
                } else {
                    stage = 2;
                    let (wn, len) = ds.length(&list, p).ok().unwrap();
                    win = wn;
                    left = len;
                    let mut best: Option<(usize, u32)> = None;
                    for (i, o) in opts.iter().enumerate() {
                        if win {
                            if ds.loses_within(&o.2, o.3, len - 1).ok().unwrap() && (best.is_none() || (opts[best.unwrap().0].1 && !o.1)) { best = Some((i, 0)); }
                        } else {
                            let (_, l) = ds.length(&o.2, o.3).ok().unwrap();
                            if best.is_none_or(|(j, b)| l > b || (l == b && opts[j].1 && !o.1)) { best = Some((i, l)); }
                        }
                    }
                    pick = best.unwrap().0;
                }
                let o = opts.swap_remove(pick);
                println!("{ply} {n} {p} {} {stage} {} {} {left} {:.0}", win as u8, o.0, o.1 as u8, now_ms() - t0);
                st.push(o.0);
                list = o.2;
                p = o.3;
                ply += 1;
            }
            println!("line {} plies {}", st.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(","), st.len());
        }
        Some("playauc") => {
            // hib playauc <depth3-results.txt> <playouts> [seed] : random playouts from every
            // depth-3 position (RULE=pass for the redundant-hint rule), with every option equally
            // likely as in `gamesize` (each redundant-hint cell is one option).
            // Rows: "key value wins playouts" (value 1 = the player to move wins; wins = playouts
            // won by the player to move).
            let pass = std::env::var("RULE").is_ok_and(|r| r == "pass");
            let k: usize = args[3].parse().unwrap();
            let mut rng = Rng::new(args.get(4).map(|s| s.parse().unwrap()).unwrap_or(1));
            fn dm(v: &[u64]) -> u64 { v.iter().fold(!0u64, |a, &x| a & x) & ((1u64 << N) - 1) }
            let mut seen = std::collections::HashSet::new();
            for l in std::fs::read_to_string(&args[2]).unwrap().lines() {
                let f: Vec<&str> = l.split(' ').collect();
                if !seen.insert(f[0].to_string()) { continue; }
                let stones: Vec<u8> = f[6].split(',').map(|x| x.parse().unwrap()).collect();
                let mut start = u.to_list(&u.root());
                for &c in &stones { start.retain(|x| (x >> c) & 1 == 1); }
                let mut wins = 0;
                for _ in 0..k {
                    let mut list = start.clone();
                    let mut placed = stones.len() as i32;
                    let mut ply = 0;
                    while list.len() > 1 {
                        let lg = legal(&list_counts(&list), list.len());
                        let p = if pass { dm(&list).count_ones() as i32 - placed } else { 0 };
                        let c = rng.below(lg.len() + p.max(0) as usize);
                        if c < lg.len() { let c = lg[c]; list.retain(|x| (x >> c) & 1 == 1); }
                        placed += 1;
                        ply += 1;
                    }
                    wins += ply % 2;
                }
                println!("{} {} {wins} {k}", f[0], f[1]);
            }
        }
        Some("gamesize") => {
            // hib gamesize <games> [seed]: random games (every option equally likely, a pass
            // counting as one option) under both rules. Prints per rule the game-length
            // distribution and the mean number of options by ply.
            let games: usize = args[2].parse().unwrap();
            let mut rng = Rng::new(args.get(3).map(|s| s.parse().unwrap()).unwrap_or(1));
            fn dm(v: &[u64]) -> u64 { v.iter().fold(!0u64, |a, &x| a & x) & ((1u64 << N) - 1) }
            for rule in 0..2 {
                let mut len_hist = vec![0u64; 130];
                let mut br_sum = vec![0f64; 130];
                let mut br_cnt = vec![0u64; 130];
                let mut passes = 0u64;
                for _ in 0..games {
                    let mut list = u.to_list(&u.root());
                    let mut ply = 0;
                    let mut stones = 0i32;
                    while list.len() > 1 {
                        let cnt = list_counts(&list);
                        let lg = legal(&cnt, list.len());
                        let p = if rule == 1 { dm(&list).count_ones() as i32 - stones } else { 0 };
                        let opts = lg.len() + p.max(0) as usize;
                        br_sum[ply] += opts as f64;
                        br_cnt[ply] += 1;
                        let k = rng.below(opts);
                        if k < lg.len() {
                            let c = lg[k];
                            list.retain(|x| (x >> c) & 1 == 1);
                        } else {
                            passes += 1;
                        }
                        stones += 1;
                        ply += 1;
                    }
                    len_hist[ply] += 1;
                }
                let total: u64 = len_hist.iter().enumerate().map(|(i, &c)| i as u64 * c).sum();
                println!("rule {rule} games {games} mean_plies {:.3} passes_per_game {:.3}", total as f64 / games as f64, passes as f64 / games as f64);
                println!("  length {}", len_hist.iter().enumerate().filter(|(_, &c)| c > 0).map(|(i, c)| format!("{i}:{c}")).collect::<Vec<_>>().join(" "));
                println!("  options_by_ply {}", (0..130).filter(|&i| br_cnt[i] > 0).map(|i| format!("{i}:{:.2}", br_sum[i] / br_cnt[i] as f64)).collect::<Vec<_>>().join(" "));
            }
        }
        Some("ppgames") => {
            // hib ppgames <known-all.txt> <book.bin> <games> [seed] : games sampled from the
            // perfect-play tree (RULE=pass for the redundant-hint rule). Moves leading to the same
            // position (same determined-black set D and, under the redundant-hint rule, the same
            // number p of redundant hints left) count as one option. The side to move, if winning,
            // picks uniformly among its winning options; if losing, uniformly among all options.
            // Every option of a winning position is valued exactly (shared table, else solved).
            // "wins" is -1 at losing positions (every option loses there).
            // Rows per position: "P game ply n p d opts wins win pick_rh rh_opt nodes ms"
            //   d = |D|, opts = options, wins = winning options (-1 when the mover loses: all lose),
            //   win = mover wins, pick_rh = the chosen option is a redundant hint,
            //   rh_opt = value of the redundant-hint option for the mover (1 win, 0 loss, -1 none),
            //   nodes / ms = search spent valuing this position's options.
            // Row per finished game: "G game plies log10_weight", weight = product of the numbers
            // of options the sampler chose from (Knuth's estimator of the number of such games).
            // ERR_PLY=k (even): the first player makes one error at ply k, choosing uniformly among
            // its losing options; from then on the second player is the winning side.
            use std::sync::Arc;
            let err_ply: Option<usize> = std::env::var("ERR_PLY").ok().map(|v| v.parse().unwrap());
            let pass = std::env::var("RULE").is_ok_and(|r| r == "pass");
            let games: usize = args[4].parse().unwrap();
            let mut rng = Rng::new(args.get(5).map(|s| s.parse().unwrap()).unwrap_or(1));
            let known = Arc::new(Known::new(3000));
            known.load_text(&std::fs::read_to_string(&args[2]).unwrap());
            known.load_bin(&std::fs::read(&args[3]).unwrap(), pass);
            eprintln!("known {} / {}", known.map.read().unwrap().len(), known.pmap.read().unwrap().len());
            let mut sv = Solver::new(24).with_symmetry();
            sv.known = Some(known.clone());
            let sym = Sym::new();
            fn dm(v: &[u64]) -> u64 { v.iter().fold(!0u64, |a, &x| a & x) & ((1u64 << N) - 1) }
            use std::io::Write;
            let out = std::io::stdout();
            for g in 0..games {
                let mut list = u.to_list(&u.root());
                let mut p = 0i32;
                let mut ply = 0;
                let mut logw = 0f64;
                let mut skipped = false;
                while list.len() > 1 {
                    let t0 = now_ms();
                    let nodes0 = sv.nodes;
                    let n = list.len();
                    let d0 = dm(&list);
                    // options: (is_rh, child, passes left in the child), deduplicated by (D, p)
                    let mut opts: Vec<(bool, Vec<u64>, i32)> = Vec::new();
                    if pass && p > 0 { opts.push((true, list.clone(), p - 1)); }
                    let cnt = list_counts(&list);
                    let mut seen = std::collections::HashSet::new();
                    for c in legal(&cnt, n) {
                        let kid: Vec<u64> = list.iter().copied().filter(|x| (x >> c) & 1 == 1).collect();
                        let kd = dm(&kid);
                        if !seen.insert(kd) { continue; }
                        let p2 = if pass { p + kd.count_ones() as i32 - d0.count_ones() as i32 - 1 } else { 0 };
                        opts.push((false, kid, p2));
                    }
                    // value of an option for the player who moves into it: true = that move wins
                    let wins_of = |o: &(bool, Vec<u64>, i32), sv: &mut Solver| -> bool {
                        if o.1.len() <= 1 { return true; }
                        let key = sym.canon(dm(&o.1));
                        let hit = if pass { known.pmap.read().unwrap().get(&(key, o.2)).copied() } else { known.map.read().unwrap().get(&key).copied() };
                        let child_wins = match hit { Some(v) => v, None => if pass { sv.win_pass(&o.1, o.2).ok().unwrap() } else { sv.win(&o.1).ok().unwrap() } };
                        !child_wins
                    };
                    // on a perfect-play path the side to move wins exactly at even plies (the root
                    // is a first-player win), so losing positions need no search; at winning
                    // positions every option is valued and a winning one must exist (checked).
                    let (win, wins, rh_opt, pick);
                    let winning_side = if err_ply.is_some_and(|k| ply > k) { 1 } else { 0 };
                    if err_ply == Some(ply) {
                        let vals: Vec<bool> = opts.iter().map(|o| wins_of(o, &mut sv)).collect();
                        let w: Vec<usize> = (0..opts.len()).filter(|&i| vals[i]).collect();
                        let l: Vec<usize> = (0..opts.len()).filter(|&i| !vals[i]).collect();
                        if l.is_empty() { eprintln!("game {g}: no losing option at ply {ply}, game skipped"); skipped = true; break; }
                        win = true;
                        wins = w.len() as i32;
                        rh_opt = if opts[0].0 { vals[0] as i32 } else { -1 };
                        pick = l[rng.below(l.len())];
                        logw += (l.len() as f64).log10();
                    } else if ply % 2 != winning_side {
                        win = false; wins = -1i32;
                        rh_opt = if opts[0].0 { 0 } else { -1 };
                        pick = rng.below(opts.len());
                        logw += (opts.len() as f64).log10();
                    } else {
                        let vals: Vec<bool> = opts.iter().map(|o| wins_of(o, &mut sv)).collect();
                        let w: Vec<usize> = (0..opts.len()).filter(|&i| vals[i]).collect();
                        assert!(!w.is_empty(), "game {g} ply {ply}: no winning option in a position that should be won");
                        win = true;
                        rh_opt = if opts[0].0 { vals[0] as i32 } else { -1 };
                        wins = w.len() as i32;
                        pick = w[rng.below(w.len())];
                        logw += (w.len() as f64).log10();
                    }
                    let o = opts.swap_remove(pick);
                    let mut lk = out.lock();
                    writeln!(lk, "P {g} {ply} {n} {p} {} {} {wins} {} {} {rh_opt} {} {:.0}", d0.count_ones(), opts.len() + 1, win as u8, o.0 as u8, sv.nodes - nodes0, now_ms() - t0).unwrap();
                    lk.flush().unwrap();
                    list = o.1;
                    p = o.2;
                    ply += 1;
                }
                if skipped { continue; }
                let mut lk = out.lock();
                writeln!(lk, "G {g} {ply} {logw:.4}").unwrap();
                lk.flush().unwrap();
            }
        }
        Some("isocost") => {
            // hib isocost <positions> [tt_bits] : solve each position under both rules with the
            // same solver settings, a fresh table per solve, no shared table, one thread.
            // positions: lines "key stones" (stones comma-separated). Rows:
            // "key n p std_value std_nodes std_ms rh_value rh_nodes rh_ms" (value 1 = mover wins).
            use std::io::Write;
            let bits: u32 = args.get(3).map(|s| s.parse().unwrap()).unwrap_or(24);
            fn dm(v: &[u64]) -> u64 { v.iter().fold(!0u64, |a, &x| a & x) & ((1u64 << N) - 1) }
            for l in std::fs::read_to_string(&args[2]).unwrap().lines() {
                let mut f = l.split(' ');
                let (Some(key), Some(st)) = (f.next(), f.next()) else { continue };
                let stones: Vec<u8> = st.split(',').map(|x| x.parse().unwrap()).collect();
                let mut list = u.to_list(&u.root());
                for &c in &stones { list.retain(|x| (x >> c) & 1 == 1); }
                let p = dm(&list).count_ones() as i32 - stones.len() as i32;
                let mut sv = Solver::new(bits).with_symmetry();
                let t = now_ms();
                let a = sv.win(&list).ok().unwrap();
                let (an, at) = (sv.nodes, now_ms() - t);
                let mut sv = Solver::new(bits).with_symmetry();
                let t = now_ms();
                let b = sv.win_pass(&list, p).ok().unwrap();
                let (bn, bt) = (sv.nodes, now_ms() - t);
                println!("{key} {} {p} {} {an} {at:.0} {} {bn} {bt:.0}", list.len(), a as u8, b as u8);
                std::io::stdout().flush().unwrap();
            }
        }
        Some("plines") => {
            // hib plines <count> <nmin> <nmax> [seed]: best lines (fastest win / longest
            // resistance) from random-game samples under the pass rule. One row per ply:
            // "sample ply_from_start plies_left mover_wins is_pass passes_left n regular_move_also_wins"
            let count: usize = args[2].parse().unwrap();
            let nmin: usize = args[3].parse().unwrap();
            let nmax: usize = args[4].parse().unwrap();
            let mut rng = Rng::new(args.get(5).map(|s| s.parse().unwrap()).unwrap_or(1));
            let mut ds = hib_engine::dist::Dist::new(true);
            fn dm(v: &[u64]) -> u64 { v.iter().fold(!0u64, |a, &x| a & x) & ((1u64 << N) - 1) }
            let mut done = 0;
            while done < count {
                let mut st: Vec<u8> = Vec::new();
                let mut list = u.to_list(&u.root());
                while list.len() > nmax {
                    let cnt = list_counts(&list);
                    let lg = legal(&cnt, list.len());
                    let c = lg[rng.below(lg.len())];
                    st.push(c);
                    list.retain(|x| (x >> c) & 1 == 1);
                }
                if list.len() < nmin { continue; }
                let mut p = dm(&list).count_ones() as i32 - st.len() as i32;
                let mut cur = list.clone();
                let mut ply = 0;
                while cur.len() > 1 {
                    let n = cur.len();
                    let d0 = dm(&cur);
                    let (win, len) = ds.length(&cur, p).ok().unwrap();
                    // options: pass first, then regular moves smallest child first
                    let mut opts: Vec<(bool, Vec<u64>, i32)> = Vec::new();
                    if p > 0 { opts.push((true, cur.clone(), p - 1)); }
                    let cnt = list_counts(&cur);
                    let mut lg = legal(&cnt, n);
                    lg.sort_unstable_by_key(|&c| cnt[c as usize]);
                    for c in lg {
                        let kid: Vec<u64> = cur.iter().copied().filter(|x| (x >> c) & 1 == 1).collect();
                        let p2 = p + dm(&kid).count_ones() as i32 - d0.count_ones() as i32 - 1;
                        opts.push((false, kid, p2));
                    }
                    // prefer a regular move among equally good options, so a pass is counted
                    // only where it is strictly needed (or strictly best for the loser)
                    let mut pick: Option<usize> = None;
                    if win {
                        for (i, o) in opts.iter().enumerate() {
                            if ds.loses_within(&o.1, o.2, len - 1).ok().unwrap() && (pick.is_none() || opts[pick.unwrap()].0) { pick = Some(i); if !o.0 { break; } }
                        }
                    } else {
                        let mut best = 0;
                        for (i, o) in opts.iter().enumerate() {
                            let (_, l) = ds.length(&o.1, o.2).ok().unwrap();
                            if l + 1 > best || (l + 1 == best && pick.is_some_and(|j: usize| opts[j].0) && !o.0) { best = l + 1; pick = Some(i); }
                        }
                    }
                    // does some regular move also win (ignoring length)?
                    let reg_win = win && opts.iter().any(|o| !o.0 && !ds.length(&o.1, o.2).ok().unwrap().0);
                    let o = opts.swap_remove(pick.unwrap());
                    println!("{done} {ply} {len} {} {} {p} {n} {}", win as u8, o.0 as u8, reg_win as u8);
                    cur = o.1;
                    p = o.2;
                    ply += 1;
                }
                done += 1;
                if done % 50 == 0 { eprintln!("{done} samples"); }
            }
        }
        Some("lengths") => {
            // hib lengths <count> <nmin> <nmax> [seed]: random-game samples as in `moves`;
            // game length under perfect play for both rules:
            // "sample ply n passes rule mover_wins plies nodes ms"
            let count: usize = args[2].parse().unwrap();
            let nmin: usize = args[3].parse().unwrap();
            let nmax: usize = args[4].parse().unwrap();
            let mut rng = Rng::new(args.get(5).map(|s| s.parse().unwrap()).unwrap_or(1));
            let mut dn = hib_engine::dist::Dist::new(false);
            let mut dp = hib_engine::dist::Dist::new(true);
            let mut done = 0;
            while done < count {
                let mut st: Vec<u8> = Vec::new();
                let mut list = u.to_list(&u.root());
                while list.len() > nmax {
                    let cnt = list_counts(&list);
                    let lg = legal(&cnt, list.len());
                    let c = lg[rng.below(lg.len())];
                    st.push(c);
                    list.retain(|x| (x >> c) & 1 == 1);
                }
                if list.len() < nmin { continue; }
                let p = list.iter().fold(!0u64, |a, &x| a & x).count_ones() as i32 - st.len() as i32;
                for (rule, ds) in [(0, &mut dn), (1, &mut dp)] {
                    let (n0, t) = (ds.nodes, now_ms());
                    let (w, l) = ds.length(&list, p).ok().unwrap();
                    println!("{done} {} {} {p} {rule} {} {l} {} {:.1}", st.len(), list.len(), w as u8, ds.nodes - n0, now_ms() - t);
                }
                done += 1;
                if done % 100 == 0 { eprintln!("{done} samples"); }
            }
        }
        Some("moves") => {
            // hib moves <count> <nmin> <nmax> [seed]: sample positions from random games
            // (uniform legal moves) with nmin <= |S| <= nmax, and solve every move under both
            // rules. Prints one row per move:
            // "sample rule ply n p_parent dD child_n is_pass child_value parent_value"
            // (values for the player to move; rule 0 normal, 1 pass).
            let count: usize = args[2].parse().unwrap();
            let nmin: usize = args[3].parse().unwrap();
            let nmax: usize = args[4].parse().unwrap();
            let mut rng = Rng::new(args.get(5).map(|s| s.parse().unwrap()).unwrap_or(1));
            let mut sv = Solver::new(23).with_symmetry();
            fn dm(v: &[u64]) -> u64 { v.iter().fold(!0u64, |a, &x| a & x) & ((1u64 << N) - 1) }
            let mut done = 0;
            while done < count {
                let mut st: Vec<u8> = Vec::new();
                let mut list = u.to_list(&u.root());
                while list.len() > nmax {
                    let cnt = list_counts(&list);
                    let lg = legal(&cnt, list.len());
                    let c = lg[rng.below(lg.len())];
                    st.push(c);
                    list.retain(|x| (x >> c) & 1 == 1);
                }
                if list.len() < nmin { continue; }
                let n = list.len();
                let cnt = list_counts(&list);
                let d0 = dm(&list);
                let p0 = d0.count_ones() as i32 - st.len() as i32;
                let lg = legal(&cnt, n);
                let mut kids: Vec<(u8, Vec<u64>, u64)> = Vec::new();
                for &c in &lg {
                    let kid: Vec<u64> = list.iter().copied().filter(|x| (x >> c) & 1 == 1).collect();
                    let kd = dm(&kid);
                    kids.push((c, kid, kd));
                }
                for rule in 0..2 {
                    let mut rows = Vec::new();
                    let mut any_loss = false;
                    for (_, kid, kd) in &kids {
                        let dd = kd.count_ones() as i32 - d0.count_ones() as i32;
                        let v = if rule == 0 { sv.win(kid).ok().unwrap() } else { sv.win_pass(kid, p0 + dd - 1).ok().unwrap() };
                        any_loss |= !v;
                        rows.push((dd, kid.len(), 0, v));
                    }
                    if rule == 1 && p0 > 0 {
                        let v = sv.win_pass(&list, p0 - 1).ok().unwrap();
                        any_loss |= !v;
                        rows.push((0, n, 1, v));
                    }
                    for (dd, kn, ip, v) in rows {
                        println!("{done} {rule} {} {n} {p0} {dd} {kn} {ip} {} {}", st.len(), v as u8, any_loss as u8);
                    }
                }
                done += 1;
                if done % 50 == 0 { eprintln!("{done} samples"); }
            }
        }
        Some("bookcov") => {
            // book coverage after the book ends: for every depth-3 position the book
            // records as lost, does the book give a winning reply to each move?
            // args: <book.bin> <positions.txt>; RULE=pass for the pass rule
            let pass = std::env::var("RULE").is_ok_and(|r| r == "pass");
            let known = Known::new(0);
            known.load_bin(&std::fs::read(&args[2]).unwrap(), pass);
            let sym = Sym::new();
            let (mut lost3, mut replies, mut found) = (0, 0, 0);
            let t = now_ms();
            for l in std::fs::read_to_string(&args[3]).unwrap().lines() {
                let f: Vec<&str> = l.split(' ').collect();
                if f[0] != "3" {
                    continue;
                }
                let st: Vec<u8> = f[3].split(',').map(|x| x.parse().unwrap()).collect();
                if book_choose(&u, &st, pass, &known, &sym, 1).is_some() {
                    continue;
                }
                lost3 += 1;
                let list = u.to_list(&u.play(&st));
                let cnt = list_counts(&list);
                let dm: u64 = (0..N).filter(|&c| cnt[c] as usize == list.len()).fold(0, |m, c| m | 1u64 << c);
                let stm = st.iter().fold(0u64, |m, &c| m | 1u64 << c);
                let mut moves: Vec<u8> = legal(&cnt, list.len());
                if pass && dm.count_ones() as usize > st.len() {
                    moves.extend((0..N as u8).find(|&c| dm >> c & 1 == 1 && stm >> c & 1 == 0));
                }
                for c in moves {
                    let mut s2 = st.clone();
                    s2.push(c);
                    replies += 1;
                    if book_choose(&u, &s2, pass, &known, &sym, 1).is_some() {
                        found += 1;
                    }
                }
                if lost3 % 200 == 0 {
                    eprintln!("{lost3} positions, {found}/{replies} replies covered, {:.0} s", (now_ms() - t) / 1000.0);
                }
            }
            println!("lost depth-3 positions {lost3}; positions after the next move {replies}; book has a winning reply for {found}");
        }
        Some("bench") => {
            let seqs: [&[u8]; 6] = [&[14, 18, 0, 60], &[31, 4, 56, 19], &[11, 59, 56, 57], &[4, 3, 34, 18], &[55, 44, 49, 1], &[23, 31, 2, 1]];
            let mut tot = 0.0;
            for s in seqs {
                let list = u.to_list(&u.play(s));
                let mut sv = if std::env::var("SYM").is_ok() { Solver::new(22).with_symmetry() } else { Solver::new(22) };
                let t = now_ms();
                let w = sv.win(&list).ok();
                let ms = now_ms() - t;
                tot += ms;
                println!("n={} win={:?} nodes={} ms={:.0}", list.len(), w, sv.nodes, ms);
            }
            println!("total {:.0} ms", tot);
        }
        Some("match") => {
            let a = args[2].clone();
            let b = args[3].clone();
            let games: usize = args[4].parse().unwrap();
            let ms: f64 = args[5].parse().unwrap();
            let offset: u64 = args.get(6).map(|s| s.parse().unwrap()).unwrap_or(0);
            let mut a_wins = 0;
            for g in 0..games {
                let a_first = g % 2 == 0;
                let seed = 1000 + g as u64 + offset;
                let mut ma = Mcts::new(params(), seed, 22);
                let mut mb = Mcts::new(params(), seed + 7777, 22);
                let mut sa = Solver::new(22);
                let mut sb = Solver::new(22);
                let mut stones: Vec<u8> = Vec::new();
                loop {
                    let n = u.play(&stones).n();
                    if n <= 1 {
                        break;
                    }
                    let a_turn = (stones.len() % 2 == 0) == a_first;
                    let (kind, m, sv) = if a_turn { (&a, &mut ma, &mut sa) } else { (&b, &mut mb, &mut sb) };
                    let mv = if kind == "flat" {
                        flat_choose(&u, &stones, ms, seed * 31 + stones.len() as u64, sv)
                    } else {
                        let r = m.search(&u, &stones, ms);
                        if std::env::var("V").is_ok() {
                            eprintln!("  {} {} n={} mv={} proven={} v={:.2} it={} po={}", stones.len(), kind, n, r.mv, r.proven, r.value, r.iters, r.playouts);
                        }
                        r.mv
                    };
                    stones.push(mv as u8);
                }
                let a_last = ((stones.len() - 1) % 2 == 0) == a_first;
                if a_last {
                    a_wins += 1;
                }
                eprintln!("game {} a_first={} len={} winner={}", g, a_first, stones.len(), if a_last { &a } else { &b });
            }
            println!("{} vs {}: {} {}/{}", a, b, a, a_wins, games);
        }
        Some("stats") => {
            // how well do cheap signals predict the exact value?
            let samples: usize = args.get(2).map(|s| s.parse().unwrap()).unwrap_or(200);
            let (lo, hi): (usize, usize) = (env("LO", 300), env("HI", 3000));
            let mut rng = Rng::new(42);
            let mut sv = Solver::new(22);
            let mut m = Mcts::new(params(), 9, 4);
            let mut m0 = Mcts::new(Params { smart_playout: false, ..params() }, 9, 4);
            let mut got = 0;
            while got < samples {
                let mut stones: Vec<u8> = Vec::new();
                let mut s = u.root();
                while s.n() > hi {
                    let cnt = u.counts(&s);
                    let l = legal(&cnt, s.n());
                    let c = l[rng.below(l.len())];
                    stones.push(c);
                    s = u.child(&s, c as usize);
                }
                if s.n() < lo { continue; }
                let list = u.to_list(&s);
                sv.deadline = now_ms() + 20000.0;
                let Ok(w) = sv.win(&list) else { continue };
                let k = 200;
                let ps = (0..k).filter(|_| m.playout(&u, &s)).count() as f64 / k as f64;
                let pr = (0..k).filter(|_| m0.playout(&u, &s)).count() as f64 / k as f64;
                let cnt = list_counts(&list);
                let nl = legal(&cnt, list.len()).len();
                println!("{} {} {} {:.3} {:.3} {}", w as u8, list.len(), nl, ps, pr, stones.len());
                got += 1;
            }
        }
        Some("enum") => {
            // distinct positions (up to symmetry) after 1..=depth stones.
            // RULE=pass: stones may also go on free determined-black cells (passes);
            // a position is then (determined-black mask, passes left), key "D:p".
            let depth: usize = args.get(2).map(|s| s.parse().unwrap()).unwrap_or(3);
            let pass_rule = std::env::var("RULE").is_ok_and(|r| r == "pass");
            let sym = Sym::new();
            let det = |s: &Set, cnt: &[u32; N]| -> u64 { (0..N).filter(|&c| cnt[c] as usize == s.n()).fold(0u64, |m, c| m | 1u64 << c) };
            let root = u.root();
            let mut level: Vec<(String, Vec<u8>)> = vec![(if pass_rule { "0:0".to_string() } else { "0".to_string() }, Vec::new())];
            let mut out = std::fs::File::create(std::env::var("OUT").unwrap_or_else(|_| "/dev/null".into())).unwrap();
            let mut edges = std::fs::File::create(std::env::var("EDGES").unwrap_or_else(|_| "/dev/null".into())).unwrap();
            use std::io::Write;
            for d in 1..=depth {
                let t = now_ms();
                let mut seen = std::collections::HashMap::<String, (Vec<u8>, usize, u64, u64)>::new();
                for (pkey, stones) in &level {
                    let s = if stones.is_empty() { root.clone() } else { u.play(stones) };
                    let cnt = u.counts(&s);
                    let mut cells = legal(&cnt, s.n());
                    if pass_rule {
                        let dm = det(&s, &cnt);
                        let free = (0..N as u8).filter(|&c| dm >> c & 1 == 1 && !stones.contains(&c));
                        if dm.count_ones() as usize > stones.len() { cells.extend(free); }
                    }
                    for c in cells {
                        let k = u.child(&s, c as usize);
                        let kc = u.counts(&k);
                        let dk = det(&k, &kc);
                        let key = if pass_rule {
                            format!("{}:{}", sym.canon(dk), dk.count_ones() as i64 - stones.len() as i64 - 1)
                        } else {
                            sym.canon(dk).to_string()
                        };
                        writeln!(edges, "{} {} {} {}", d, pkey, key, c).unwrap();
                        let xm = (0..N).filter(|&c| kc[c] == 0).fold(0u64, |m, c| m | 1u64 << c);
                        seen.entry(key).or_insert_with(|| { let mut st = stones.clone(); st.push(c); (st, k.n(), dk, xm) });
                    }
                }
                let mut ns: Vec<usize> = seen.values().map(|v| v.1).collect();
                ns.sort_unstable();
                let q = |f: f64| ns[((ns.len() - 1) as f64 * f) as usize];
                println!("depth {} positions {} n: min {} p10 {} median {} p90 {} max {} ({:.0} s)", d, ns.len(), ns[0], q(0.1), q(0.5), q(0.9), ns[ns.len() - 1], (now_ms() - t) / 1000.0);
                for (key, (st, n, dm, xm)) in &seen {
                    // dm / xm: cells determined black / white, in the orientation of `st`
                    writeln!(out, "{} {} {} {} {} {}", d, key, n, st.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(","), dm, xm).unwrap();
                }
                level = seen.into_iter().map(|(k, v)| (k, v.0)).collect();
            }
        }
        Some("sample") => {
            // solve random positions from an `enum` dump: file depth count cap_s
            let text = std::fs::read_to_string(&args[2]).unwrap();
            let depth: usize = args[3].parse().unwrap();
            let count: usize = args[4].parse().unwrap();
            let cap: f64 = args[5].parse().unwrap();
            let rows: Vec<(usize, Vec<u8>)> = text.lines().filter_map(|l| {
                let f: Vec<&str> = l.split(' ').collect();
                if f[0].parse::<usize>().ok()? != depth { return None; }
                Some((f[2].parse().ok()?, f[3].split(',').map(|x| x.parse().unwrap()).collect()))
            }).collect();
            let mut rng = Rng::new(env("SEED", 7u64));
            let mut sv = Solver::new(22);
            for _ in 0..count {
                let (n, stones) = &rows[rng.below(rows.len())];
                let list = u.to_list(&u.play(stones));
                sv.nodes = 0;
                sv.deadline = now_ms() + cap * 1000.0;
                let t = now_ms();
                let r = match sv.win(&list) { Ok(true) => "win", Ok(false) => "loss", Err(_) => "timeout" };
                println!("{} {} {} {:.0} {}", n, r, sv.nodes, now_ms() - t, stones.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(","));
            }
        }
        Some("perfect") => {
            // play sampled positions to the end with perfect play:
            // the winner plays the solver's winning move, the loser keeps the most solutions.
            let text = std::fs::read_to_string(&args[2]).unwrap();
            let depth: usize = args[3].parse().unwrap();
            let count: usize = args[4].parse().unwrap();
            let cap: f64 = args[5].parse().unwrap();
            let rows: Vec<Vec<u8>> = text.lines().filter_map(|l| {
                let f: Vec<&str> = l.split(' ').collect();
                if f[0].parse::<usize>().ok()? != depth { return None; }
                Some(f[3].split(',').map(|x| x.parse().unwrap()).collect())
            }).collect();
            let mut rng = Rng::new(env("SEED", 7u64));
            let mut sv = Solver::new(23);
            'game: for _ in 0..count {
                let mut stones = rows[rng.below(rows.len())].clone();
                let n0 = u.play(&stones).n();
                let start = stones.len();
                let t = now_ms();
                let mut first_winner_to_move = None;
                loop {
                    let list = u.to_list(&u.play(&stones));
                    if list.len() <= 1 { break; }
                    sv.deadline = t + cap * 1000.0;
                    let Ok(bm) = sv.best_move(&list) else {
                        println!("{} timeout", n0);
                        continue 'game;
                    };
                    if first_winner_to_move.is_none() { first_winner_to_move = Some(bm.is_some()); }
                    let c = match bm {
                        Some(c) => c,
                        None => {
                            let cnt = list_counts(&list);
                            *legal(&cnt, list.len()).iter().max_by_key(|&&c| cnt[c as usize]).unwrap() as usize
                        }
                    };
                    stones.push(c as u8);
                }
                println!("{} {} total_plies {} after_start {} ms {:.0}", n0, if first_winner_to_move == Some(true) { "mover_wins" } else { "mover_loses" }, stones.len(), stones.len() - start, now_ms() - t);
            }
        }
        Some("solve") => {
            // hib solve <positions> <results> <threads>
            // positions: lines "depth key n stones"; results: appended lines
            // "key value move n ms nodes stones" (value 1 = player to move wins, move -1 if lost).
            // Restartable: keys already in <results> are skipped.
            // defer.txt (next to <results>, reloaded every 5 minutes) lists positions not
            // needed for the current goal: they are postponed to the end, and a running
            // one is abandoned and requeued (its sub-results stay in the table).
            use std::io::Write;
            use std::sync::atomic::Ordering;
            use std::sync::{Arc, Mutex};
            let pos_text = std::fs::read_to_string(&args[2]).unwrap();
            let out_path = args[3].clone();
            let threads: usize = args[4].parse().unwrap();
            let done: std::collections::HashSet<String> = std::fs::read_to_string(&out_path)
                .unwrap_or_default().lines().filter_map(|l| l.split(' ').next().map(String::from)).collect();
            let jobs: Vec<(String, usize, Vec<u8>)> = pos_text.lines().filter_map(|l| {
                let f: Vec<&str> = l.split(' ').collect();
                if f.len() < 4 || done.contains(f[1]) { return None; }
                Some((f[1].to_string(), f[2].parse().ok()?, f[3].split(',').map(|x| x.parse().unwrap()).collect()))
            }).collect();
            eprintln!("{} jobs ({} already done), {} threads", jobs.len(), done.len(), threads);
            let out = Arc::new(Mutex::new(std::fs::OpenOptions::new().create(true).append(true).open(&out_path).unwrap()));
            let u = Arc::new(u);
            let jobs = Arc::new(jobs);
            // one table for every thread; large positions are shared across machines
            // through known.txt (our exports) and known-all.txt (merged, pushed back)
            let pass_rule = std::env::var("RULE").is_ok_and(|r| r == "pass");
            let shared = Arc::new(SharedTT::new(if pass_rule { 4 } else { env("TT_BITS", 25u32) }));
            let vshared = Arc::new(SharedTT2::new(if pass_rule { env("TT_BITS", 25u32) } else { 4 }));
            let known = Arc::new(Known::new(env("KNOWN_MIN", 3000usize)));
            let dir = std::path::Path::new(&out_path).parent().map(|p| p.to_path_buf()).unwrap_or_default();
            let all_path = dir.join("known-all.txt");
            let exp_path = dir.join("known.txt");
            if let Ok(t) = std::fs::read_to_string(&exp_path) { known.load_text(&t); }
            let mut all_mtime = None;
            if let Ok(t) = std::fs::read_to_string(&all_path) {
                all_mtime = std::fs::metadata(&all_path).and_then(|m| m.modified()).ok();
                eprintln!("loaded {} known positions", known.load_text(&t));
            }
            let finished = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let defer_path = dir.join("defer.txt");
            let defer = Arc::new(std::sync::RwLock::new(std::collections::HashSet::<String>::new()));
            let load_defer = {
                let defer = defer.clone();
                move |p: &std::path::Path| {
                    if let Ok(t) = std::fs::read_to_string(p) {
                        let set: std::collections::HashSet<String> = t.lines().map(String::from).collect();
                        eprintln!("defer list: {} positions", set.len());
                        *defer.write().unwrap() = set;
                    }
                }
            };
            load_defer(&defer_path);
            // order.txt: needed positions, most useful first (reloaded with defer.txt)
            let order_path = dir.join("order.txt");
            let rank = Arc::new(std::sync::RwLock::new(std::collections::HashMap::<String, usize>::new()));
            let load_order = {
                let rank = rank.clone();
                move |p: &std::path::Path| {
                    if let Ok(t) = std::fs::read_to_string(p) {
                        *rank.write().unwrap() = t.lines().enumerate().map(|(i, k)| (k.to_string(), i)).collect();
                    }
                }
            };
            load_order(&order_path);
            // unclaimed job indices; a claim takes the best-ranked one
            let queue = Arc::new(Mutex::new((0..jobs.len()).collect::<Vec<usize>>()));
            let deferred = Arc::new(Mutex::new(Vec::<usize>::new()));
            let current: Arc<Vec<Mutex<Option<String>>>> = Arc::new((0..threads).map(|_| Mutex::new(None)).collect());
            let stops: Arc<Vec<Arc<std::sync::atomic::AtomicBool>>> =
                Arc::new((0..threads).map(|_| Arc::new(std::sync::atomic::AtomicBool::new(false))).collect());
            let flush = {
                let known = known.clone();
                let exp_path = exp_path.clone();
                move || {
                    let items: Vec<(u64, bool, u32)> = std::mem::take(&mut *known.export.lock().unwrap());
                    let pitems: Vec<(u64, i32, bool, u32)> = std::mem::take(&mut *known.pexport.lock().unwrap());
                    if items.is_empty() && pitems.is_empty() { return; }
                    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&exp_path).unwrap();
                    let mut buf = String::new();
                    for (k, v, n) in items { buf.push_str(&format!("{} {} {}\n", k, v as u8, n)); }
                    for (k, p, v, n) in pitems { buf.push_str(&format!("{}:{} {} {}\n", k, p, v as u8, n)); }
                    f.write_all(buf.as_bytes()).unwrap();
                }
            };
            let poller = {
                let (known, finished, flush) = (known.clone(), finished.clone(), flush.clone());
                let (defer, current, stops, load_defer) = (defer.clone(), current.clone(), stops.clone(), load_defer.clone());
                let (queue_p, load_order, order_path) = (queue.clone(), load_order.clone(), order_path.clone());
                let mut order_mtime = std::fs::metadata(&order_path).and_then(|m| m.modified()).ok();
                let mut defer_mtime = std::fs::metadata(&defer_path).and_then(|m| m.modified()).ok();
                std::thread::spawn(move || {
                    let mut last = all_mtime;
                    while !finished.load(Ordering::SeqCst) {
                        for _ in 0..60 { if finished.load(Ordering::SeqCst) { break; } std::thread::sleep(std::time::Duration::from_secs(5)); }
                        flush();
                        let m = std::fs::metadata(&all_path).and_then(|m| m.modified()).ok();
                        if m.is_some() && m != last {
                            if let Ok(t) = std::fs::read_to_string(&all_path) { eprintln!("merged {} new known positions", known.load_text(&t)); }
                            last = m;
                        }
                        let om = std::fs::metadata(&order_path).and_then(|m| m.modified()).ok();
                        if om.is_some() && om != order_mtime { load_order(&order_path); order_mtime = om; }
                        let dm = std::fs::metadata(&defer_path).and_then(|m| m.modified()).ok();
                        if dm.is_some() && dm != defer_mtime {
                            load_defer(&defer_path);
                            defer_mtime = dm;
                            let d = defer.read().unwrap();
                            // only while needed positions remain: afterwards everything left is postponed work
                            let main_left = !queue_p.lock().unwrap().is_empty();
                            for (t, cur) in current.iter().enumerate().filter(|_| main_left) {
                                if cur.lock().unwrap().as_ref().is_some_and(|k| d.contains(k)) {
                                    stops[t].store(true, Ordering::SeqCst);
                                }
                            }
                        }
                    }
                })
            };
            let hs: Vec<_> = (0..threads).map(|t_id| {
                let (out, u, jobs) = (out.clone(), u.clone(), jobs.clone());
                let (shared, known, vshared) = (shared.clone(), known.clone(), vshared.clone());
                let (defer, deferred, current, stops) = (defer.clone(), deferred.clone(), current.clone(), stops.clone());
                let (queue, rank) = (queue.clone(), rank.clone());
                std::thread::spawn(move || {
                    let mut sv = Solver::new(4).with_symmetry();
                    sv.shared = Some(shared);
                    sv.vshared = Some(vshared);
                    sv.known = Some(known);
                    sv.stop = Some(stops[t_id].clone());
                    loop {
                        // needed positions first (best rank first), then the postponed ones
                        let claimed = {
                            let mut q = queue.lock().unwrap();
                            let d = defer.read().unwrap();
                            // postponed positions leave the main queue
                            let mut j = 0;
                            while j < q.len() {
                                if d.contains(&jobs[q[j]].0) { deferred.lock().unwrap().push(q.swap_remove(j)); } else { j += 1; }
                            }
                            let r = rank.read().unwrap();
                            let best = (0..q.len()).min_by_key(|&j| (r.get(&jobs[q[j]].0).copied().unwrap_or(usize::MAX), q[j]));
                            best.map(|j| q.swap_remove(j))
                        };
                        let i = match claimed {
                            Some(i) => i,
                            None => {
                                let mut dq = deferred.lock().unwrap();
                                if dq.is_empty() { break; }
                                dq.sort_unstable();
                                dq.remove(0)
                            }
                        };
                        let (key, n, stones) = &jobs[i];
                        *current[t_id].lock().unwrap() = Some(key.clone());
                        stops[t_id].store(false, Ordering::SeqCst);
                        let list = u.to_list(&u.play(stones));
                        let t = now_ms();
                        sv.nodes = 0;
                        sv.deadline = f64::INFINITY;
                        sv.node_limit = u64::MAX;
                        let pass_p: Option<i32> = key.split_once(':').map(|(_, p)| p.parse().unwrap());
                        let res = match pass_p { Some(p) => sv.win_pass(&list, p), None => sv.win(&list) };
                        let Ok(win) = res else {
                            eprintln!("postponed {} after {:.0} s", key, (now_ms() - t) / 1000.0);
                            deferred.lock().unwrap().push(i);
                            continue;
                        };
                        *current[t_id].lock().unwrap() = None;
                        if std::env::var("CHECK").is_ok() {
                            // cross-check against a private table without symmetry
                            let mut fresh = Solver::new(20);
                            let w2 = match pass_p { Some(p) => fresh.win_pass(&list, p), None => fresh.win(&list) }.ok().unwrap();
                            assert_eq!(win, w2, "value mismatch for {}", key);
                            eprintln!("checked {} = {}", key, win);
                        }
                        let nodes = sv.nodes;
                        let mv = if !win { -1 } else {
                            let bm = match pass_p { Some(p) => sv.best_move_pass(&list, p, stones), None => sv.best_move(&list) };
                            bm.ok().flatten().map(|c| c as i32).unwrap_or(-2)
                        };
                        let line = format!("{} {} {} {} {:.0} {} {}\n", key, win as u8, mv, n, now_ms() - t, nodes,
                            stones.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(","));
                        let mut f = out.lock().unwrap();
                        f.write_all(line.as_bytes()).unwrap();
                        f.sync_data().ok();
                    }
                })
            }).collect();
            for h in hs { h.join().unwrap(); }
            finished.store(true, Ordering::SeqCst);
            poller.join().unwrap();
            flush();
            eprintln!("all done");
        }
        Some("boards") | Some("boards4") | Some("winkeys") => {
            // boards: for every first move in the 1/12 wedge and every second move, the
            //   key of each third move. line: c1 c2 dmask xmask c3:key,...
            // boards4 <pairs>: for each "c1 c2", every third move and the key of each
            //   fourth move. line: c1 c2 c3 dmask xmask c4:key,...
            // winkeys <res files>: "position_key child_key" for each won position's winning move.
            // RULE=pass: free determined-black cells are moves too, keys are "mask:passes".
            let pass_rule = std::env::var("RULE").is_ok_and(|r| r == "pass");
            let sym = Sym::new();
            let det = |s: &Set, cnt: &[u32; N]| -> u64 { (0..N).filter(|&c| cnt[c] as usize == s.n()).fold(0u64, |m, c| m | 1u64 << c) };
            // moves from a position reached by `stones`
            let moves = |stones: &[u8], s: &Set, cnt: &[u32; N]| -> Vec<u8> {
                let mut m = legal(cnt, s.n());
                if pass_rule {
                    let dm = det(s, cnt);
                    if dm.count_ones() as usize > stones.len() {
                        m.extend((0..N as u8).filter(|&c| dm >> c & 1 == 1 && !stones.contains(&c)));
                    }
                }
                m
            };
            let key = |stones_len: usize, s: &Set, cnt: &[u32; N]| -> String {
                let dm = det(s, cnt);
                if pass_rule { format!("{}:{}", sym.canon(dm), dm.count_ones() as i64 - stones_len as i64) } else { sym.canon(dm).to_string() }
            };
            let xmask = |cnt: &[u32; N]| (0..N).filter(|&c| cnt[c] == 0).fold(0u64, |m, c| m | 1u64 << c);
            use std::io::Write;
            let stdout = std::io::stdout();
            let mut o = std::io::BufWriter::new(stdout.lock());
            let cmd = args[1].as_str();
            if cmd == "winkeys" {
                for f in &args[2..] {
                    for l in std::fs::read_to_string(f).unwrap_or_default().lines() {
                        let p: Vec<&str> = l.split(' ').collect();
                        if p.len() < 7 || p[1] != "1" { continue; }
                        let Ok(mv) = p[2].parse::<i32>() else { continue };
                        if mv < 0 { continue; }
                        let mut st: Vec<u8> = p[6].split(',').map(|x| x.parse().unwrap()).collect();
                        st.push(mv as u8);
                        let s = u.play(&st);
                        let k = u.counts(&s);
                        writeln!(o, "{} {}", p[0], key(st.len(), &s, &k)).unwrap();
                    }
                }
            } else {
                // depth-2 positions to expand: all wedge first moves (boards) or the given pairs (boards4)
                let cs = cells();
                let mut pairs: Vec<[u8; 2]> = Vec::new();
                if cmd == "boards" {
                    let root = u.root();
                    for c1 in 0..N {
                        let (x, y) = cs[c1];
                        let z = -x - y;
                        if !(x >= y && y >= z) { continue; }
                        let s1 = u.child(&root, c1);
                        let k1 = u.counts(&s1);
                        for c2 in moves(&[c1 as u8], &s1, &k1) { pairs.push([c1 as u8, c2]); }
                    }
                } else {
                    for l in std::fs::read_to_string(&args[2]).unwrap().lines() {
                        let f: Vec<u8> = l.split(' ').map(|x| x.parse().unwrap()).collect();
                        pairs.push([f[0], f[1]]);
                    }
                }
                for pr in pairs {
                    let s2 = u.play(&pr);
                    let k2 = u.counts(&s2);
                    if cmd == "boards" {
                        let parts: Vec<String> = moves(&pr, &s2, &k2).into_iter().map(|c3| {
                            let s3 = u.child(&s2, c3 as usize);
                            let k3 = u.counts(&s3);
                            format!("{}:{}", c3, key(3, &s3, &k3))
                        }).collect();
                        writeln!(o, "{} {} {} {} {}", pr[0], pr[1], det(&s2, &k2), xmask(&k2), parts.join(",")).unwrap();
                        continue;
                    }
                    for c3 in moves(&pr, &s2, &k2) {
                        let st3 = [pr[0], pr[1], c3];
                        let s3 = u.child(&s2, c3 as usize);
                        let k3 = u.counts(&s3);
                        let parts: Vec<String> = moves(&st3, &s3, &k3).into_iter().map(|c4| {
                            let s4 = u.child(&s3, c4 as usize);
                            let k4 = u.counts(&s4);
                            format!("{}:{}", c4, key(4, &s4, &k4))
                        }).collect();
                        writeln!(o, "{} {} {} {} {} {}", pr[0], pr[1], c3, det(&s3, &k3), xmask(&k3), parts.join(",")).unwrap();
                    }
                }
            }
        }
        Some("variant") => {
            // Variant rule: a stone may also go on a cell already determined black (a
            // "pass": the solution set is unchanged). Passes left = |D| - stones placed.
            // Compare values with the normal rule on random positions.
            let samples: usize = args.get(2).map(|s| s.parse().unwrap()).unwrap_or(200);
            let (lo, hi): (usize, usize) = (env("LO", 300), env("HI", 3000));
            let mut rng = Rng::new(env("SEED", 42u64));
            let mut sv = Solver::new(22);
            let mut memo: std::collections::HashMap<(Vec<u64>, u32), bool> = Default::default();
            fn dmask(v: &[u64]) -> u64 { v.iter().fold(!0u64, |a, &x| a & x) & ((1u64 << N) - 1) }
            // true = player to move wins
            fn vwin(v: &[u64], p: u32, memo: &mut std::collections::HashMap<(Vec<u64>, u32), bool>, cnt_nodes: &mut u64) -> bool {
                let n = v.len();
                if n <= 1 { return false; }
                let key = (vec![dmask(v), n as u64, v[0]], p);
                if let Some(&r) = memo.get(&key) { return r; }
                *cnt_nodes += 1;
                let cnt = list_counts(v);
                let d0 = dmask(v).count_ones();
                let mut res = cnt.iter().any(|&k| k == 1);
                if !res && p > 0 && !vwin(v, p - 1, memo, cnt_nodes) { res = true; }
                if !res {
                    let mut moves: Vec<usize> = (0..N).filter(|&c| cnt[c] > 2 && (cnt[c] as usize) < n).collect();
                    moves.sort_unstable_by_key(|&c| cnt[c]);
                    for c in moves {
                        let kid: Vec<u64> = v.iter().copied().filter(|x| (x >> c) & 1 == 1).collect();
                        let p2 = p + dmask(&kid).count_ones() - d0 - 1;
                        if !vwin(&kid, p2, memo, cnt_nodes) { res = true; break; }
                    }
                }
                memo.insert(key, res);
                res
            }
            let mut got = 0;
            let (mut same, mut n_win, mut v_win) = (0, 0, 0);
            while got < samples {
                let mut stones: Vec<u8> = Vec::new();
                let mut s = u.root();
                while s.n() > hi {
                    let cnt = u.counts(&s);
                    let l = legal(&cnt, s.n());
                    let c = l[rng.below(l.len())];
                    stones.push(c);
                    s = u.child(&s, c as usize);
                }
                if s.n() < lo { continue; }
                let list = u.to_list(&s);
                let p = dmask(&list).count_ones() - stones.len() as u32;
                sv.deadline = now_ms() + 20000.0;
                let Ok(w) = sv.win(&list) else { continue };
                let mut nodes = 0u64;
                if memo.len() > 3_000_000 { memo.clear(); }
                let wv = vwin(&list, p, &mut memo, &mut nodes);
                let wv0 = vwin(&list, 0, &mut memo, &mut nodes);
                if std::env::var("CHECK").is_ok() {
                    for pp in 0..4 {
                        let a = vwin(&list, pp, &mut memo, &mut nodes);
                        let b = sv.win_pass(&list, pp as i32).ok().unwrap();
                        assert_eq!(a, b, "mismatch n={} passes={}", list.len(), pp);
                    }
                }
                println!("{} n={} p={} normal={} variant={} variant_p0={}", got, list.len(), p, w as u8, wv as u8, wv0 as u8);
                got += 1;
                if w == wv { same += 1; }
                if w { n_win += 1; }
                if wv { v_win += 1; }
            }
            println!("samples {} same {} mover-wins normal {} variant {}", samples, same, n_win, v_win);
        }
        Some("psym") => {
            // hib psym <res file> <count> <lo> <hi>: pass-rule solve time with and without
            // symmetry-reduced keys (fresh private tables, one thread)
            let text = std::fs::read_to_string(&args[2]).unwrap();
            let count: usize = args[3].parse().unwrap();
            let (lo, hi): (usize, usize) = (args[4].parse().unwrap(), args[5].parse().unwrap());
            let mut k = 0;
            for l in text.lines() {
                let f: Vec<&str> = l.split(' ').collect();
                let n: usize = f[3].parse().unwrap();
                if n < lo || n > hi { continue; }
                let stones: Vec<u8> = f[6].split(',').map(|x| x.parse().unwrap()).collect();
                let p: i32 = f[0].split_once(':').unwrap().1.parse().unwrap();
                let list = u.to_list(&u.play(&stones));
                let mut r = Vec::new();
                for sym in [false, true] {
                    let mut sv = if sym { Solver::new(4).with_symmetry() } else { Solver::new(4) };
                    let t = now_ms();
                    let w = sv.win_pass(&list, p).ok().unwrap();
                    r.push(format!("{} {:.0}ms {}nodes", if sym { "sym" } else { "plain" }, now_ms() - t, sv.nodes));
                    let _ = w;
                }
                println!("n={} value={} | {}", n, f[1], r.join(" | "));
                k += 1;
                if k >= count { break; }
            }
        }
        Some("pwhy") => {
            // hib pwhy <pass res file> <count> <lo> <hi>: for lost pass-rule positions, how
            // does the opponent answer each move (pass or regular, passes available), and what
            // would the same position after the move be worth under the normal rule?
            let text = std::fs::read_to_string(&args[2]).unwrap();
            let count: usize = args[3].parse().unwrap();
            let (lo, hi): (usize, usize) = (args[4].parse().unwrap(), args[5].parse().unwrap());
            fn dmask(v: &[u64]) -> u64 { v.iter().fold(!0u64, |a, &x| a & x) & ((1u64 << N) - 1) }
            let mut k = 0;
            let mut tot = std::collections::BTreeMap::<String, usize>::new();
            for l in text.lines() {
                let f: Vec<&str> = l.split(' ').collect();
                let n: usize = f[3].parse().unwrap();
                if f[1] != "0" || n < lo || n > hi { continue; }
                let stones: Vec<u8> = f[6].split(',').map(|x| x.parse().unwrap()).collect();
                let list = u.to_list(&u.play(&stones));
                let d0 = dmask(&list).count_ones() as i32;
                let p0 = d0 - stones.len() as i32;
                let cnt = list_counts(&list);
                let mut sv = Solver::new(22);
                let mut stat = std::collections::BTreeMap::<String, usize>::new();
                for c in (0..N).filter(|&c| cnt[c] > 0 && (cnt[c] as usize) < n) {
                    let kid: Vec<u64> = list.iter().copied().filter(|x| (x >> c) & 1 == 1).collect();
                    let p1 = p0 + dmask(&kid).count_ones() as i32 - d0 - 1;
                    let mut st2 = stones.clone(); st2.push(c as u8);
                    let reply = sv.best_move_pass(&kid, p1, &st2).ok().flatten();
                    let kc = list_counts(&kid);
                    let kind = match reply {
                        None => "no-reply(bug?)".to_string(),
                        Some(r) if kc[r] as usize == kid.len() => format!("reply=pass(passes {})", p1),
                        Some(r) if kc[r] == 1 => "reply=immediate-win".to_string(),
                        Some(_) => format!("reply=regular(passes {})", p1),
                    };
                    let normal = if sv.win(&kid).ok().unwrap() { "normal:opponent-wins-too" } else { "normal:opponent-would-lose" };
                    *stat.entry(format!("{} {}", kind, normal)).or_default() += 1;
                }
                println!("position n={} passes={} moves={}", n, p0, stat.values().sum::<usize>());
                for (s, c) in &stat { println!("   {:>3}  {}", c, s); *tot.entry(s.clone()).or_default() += c; }
                k += 1;
                if k >= count { break; }
            }
            println!("TOTAL");
            for (s, c) in &tot { println!("   {:>4}  {}", c, s); }
        }
        Some("shape") => {
            // hib shape <pass res> <normal res...>: search shape under both rules for the same
            // positions (fresh tables): how many moves are tried before a win is found, how many
            // must be refuted at lost nodes.
            let ptext = std::fs::read_to_string(&args[2]).unwrap();
            let count: usize = env("COUNT", 4);
            let mut normal_ms = std::collections::HashMap::<String, f64>::new();
            for f in &args[3..] {
                for l in std::fs::read_to_string(f).unwrap_or_default().lines() {
                    let p: Vec<&str> = l.split(' ').collect();
                    normal_ms.insert(p[0].to_string(), p[4].parse().unwrap_or(1e9));
                }
            }
            let mut k = 0;
            for l in ptext.lines() {
                let f: Vec<&str> = l.split(' ').collect();
                let (dk, pp) = f[0].split_once(':').unwrap();
                let pp: i32 = pp.parse().unwrap();
                // positions that the normal rule solved in under 30 s, so both runs stay short
                let (tlo, thi): (f64, f64) = (env("TLO", 0.0), env("THI", 30000.0));
                let tn = normal_ms.get(dk).copied().unwrap_or(1e9);
                if tn < tlo || tn > thi { continue; }
                let n: usize = f[3].parse().unwrap();
                if n < 15000 { continue; }
                let stones: Vec<u8> = f[6].split(',').map(|x| x.parse().unwrap()).collect();
                let list = u.to_list(&u.play(&stones));
                for rule in ["normal", "pass"] {
                    let mut sv = Solver::new(23);
                    let t = now_ms();
                    let v = if rule == "pass" { sv.win_pass(&list, pp) } else { sv.win(&list) }.ok().unwrap();
                    let sh = sv.shape;
                    let wins = sh[0] + sh[1];
                    println!("n={} {:6} value={} nodes={} ms={:.0} | win nodes {} (immediate {}), tries per searched win {:.2} | lost nodes {}, moves refuted per lost node {:.1} | pass tries {}",
                        n, rule, v as u8, sv.nodes, now_ms() - t, wins, sh[0], sh[2] as f64 / sh[1].max(1) as f64, sh[3], sh[4] as f64 / sh[3].max(1) as f64, sh[5]);
                }
                k += 1;
                if k >= count { break; }
            }
        }
        Some("pcheck") => {
            // hib pcheck <res file> <max n>: re-solve recorded pass-rule results with a plain
            // reference search (explicit lists, no symmetry, no shared tables) and compare.
            let text = std::fs::read_to_string(&args[2]).unwrap();
            let max_n: usize = args[3].parse().unwrap();
            fn dmask(v: &[u64]) -> u64 { v.iter().fold(!0u64, |a, &x| a & x) & ((1u64 << N) - 1) }
            fn rwin(v: &[u64], p: i32, memo: &mut std::collections::HashMap<(u64, i32), bool>) -> bool {
                let n = v.len();
                if n <= 1 { return false; }
                let d = dmask(v);
                if let Some(&r) = memo.get(&(d, p)) { return r; }
                let cnt = list_counts(v);
                let mut res = cnt.contains(&1);
                if !res && p > 0 && !rwin(v, p - 1, memo) { res = true; }
                if !res {
                    for c in 0..N {
                        if cnt[c] == 0 || cnt[c] as usize == n { continue; }
                        let kid: Vec<u64> = v.iter().copied().filter(|x| (x >> c) & 1 == 1).collect();
                        let p2 = p + dmask(&kid).count_ones() as i32 - d.count_ones() as i32 - 1;
                        if !rwin(&kid, p2, memo) { res = true; break; }
                    }
                }
                memo.insert((d, p), res);
                res
            }
            let (mut ok, mut bad) = (0, 0);
            for l in text.lines() {
                let f: Vec<&str> = l.split(' ').collect();
                let n: usize = f[3].parse().unwrap();
                if n > max_n { continue; }
                let stones: Vec<u8> = f[6].split(',').map(|x| x.parse().unwrap()).collect();
                let list = u.to_list(&u.play(&stones));
                let p = dmask(&list).count_ones() as i32 - stones.len() as i32;
                let mut memo = Default::default();
                let r = rwin(&list, p, &mut memo);
                let rec = f[1] == "1";
                if r == rec { ok += 1 } else { bad += 1; println!("MISMATCH {} n={} recorded={} reference={}", f[0], n, rec, r); }
            }
            println!("checked {} agree {} disagree {}", ok + bad, ok, bad);
        }
        _ => eprintln!("usage: hib bench | hib match <mcts|flat> <mcts|flat> <games> <ms> [offset]"),
    }
}
