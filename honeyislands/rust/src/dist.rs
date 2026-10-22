//! Game length under perfect play: the winner finishes as fast as possible,
//! the loser delays the end as long as possible. Both rules.
//!
//! `win_within(S, k)`: the player to move can win within k plies (their own
//! winning move included). `lose_within(T, j)`: the player to move in T loses
//! within j plies whatever they do. A win always takes an odd number of plies,
//! a loss an even number, so the exact length is the first k (iterative
//! deepening) for which one of the two holds.

use crate::{now_ms, Abort, Sym, N};
use std::collections::HashMap;

const NONE: u8 = 255;

pub struct Dist {
    col: Vec<u64>,
    w: usize,
    ibuf: Vec<u32>,
    vbuf: Vec<u64>,
    sym: Sym,
    pass: bool,
    /// key -> [largest k known to fail win_within, smallest k known to pass it,
    ///         largest j known to fail lose_within, smallest j known to pass it]
    tt: HashMap<(u64, i32), [u8; 4]>,
    /// plain win/loss values (true = the player to move wins), used to prune
    val: HashMap<(u64, i32), bool>,
    pub nodes: u64,
    pub vnodes: u64,
    pub deadline: f64,
}

/// Per-node facts: counts, determined masks, whether a size-1 child exists.
struct Node {
    cnt: [u32; N],
    dmask: u64,
    xmask: u64,
    has1: bool,
}

impl Dist {
    pub fn new(pass: bool) -> Self {
        Dist { col: Vec::new(), w: 0, ibuf: Vec::new(), vbuf: Vec::new(), sym: Sym::new(), pass, tt: HashMap::new(), val: HashMap::new(), nodes: 0, vnodes: 0, deadline: f64::INFINITY }
    }

    pub fn clear(&mut self) {
        self.tt.clear();
        self.val.clear();
    }

    fn prepare(&mut self, sols: &[u64]) {
        let n = sols.len();
        let w = n.div_ceil(64).max(1);
        self.w = w;
        self.col.clear();
        self.col.resize(N * w, 0);
        for (i, &s) in sols.iter().enumerate() {
            let mut a = s;
            while a != 0 {
                self.col[a.trailing_zeros() as usize * w + i / 64] |= 1u64 << (i % 64);
                a &= a - 1;
            }
        }
        let depth_max = 72;
        self.ibuf.clear();
        self.ibuf.resize(depth_max * w, 0);
        self.vbuf.clear();
        self.vbuf.resize(depth_max * w, 0);
        for j in 0..w {
            self.ibuf[j] = j as u32;
            self.vbuf[j] = !0;
        }
        if n % 64 != 0 {
            self.vbuf[w - 1] = (1u64 << (n % 64)) - 1;
        }
    }

    fn node(&self, d: usize, len: usize, n: usize, bmask: u64, wmask: u64) -> Node {
        let w = self.w;
        let idx = &self.ibuf[d * w..d * w + len];
        let val = &self.vbuf[d * w..d * w + len];
        let mut nd = Node { cnt: [0; N], dmask: bmask, xmask: wmask, has1: false };
        for c in 0..N {
            let bit = 1u64 << c;
            if bmask & bit != 0 {
                nd.cnt[c] = n as u32;
                continue;
            }
            if wmask & bit != 0 {
                continue;
            }
            let col = &self.col[c * w..(c + 1) * w];
            let mut k = 0u32;
            for j in 0..len {
                k += (val[j] & col[idx[j] as usize]).count_ones();
            }
            nd.cnt[c] = k;
            if k as usize == n {
                nd.dmask |= bit;
            } else if k == 0 {
                nd.xmask |= bit;
            } else if k == 1 {
                nd.has1 = true;
            }
        }
        nd
    }

    /// child after a stone on c, written to level d + 1; returns its length
    fn kid(&mut self, d: usize, len: usize, c: usize) -> usize {
        let w = self.w;
        let (ilo, ihi) = self.ibuf.split_at_mut((d + 1) * w);
        let (vlo, vhi) = self.vbuf.split_at_mut((d + 1) * w);
        let idx = &ilo[d * w..d * w + len];
        let val = &vlo[d * w..d * w + len];
        let col = &self.col[c * w..(c + 1) * w];
        let mut kk = 0;
        for j in 0..len {
            let x = val[j] & col[idx[j] as usize];
            if x != 0 {
                ihi[kk] = idx[j];
                vhi[kk] = x;
                kk += 1;
            }
        }
        kk
    }

    fn tick(&mut self) -> Result<(), Abort> {
        self.nodes += 1;
        if self.nodes & 1023 == 0 && now_ms() > self.deadline {
            return Err(Abort);
        }
        Ok(())
    }

    /// regular moves (cells with 0 < count < n), smallest child first
    fn moves(nd: &Node, n: usize) -> ([u8; N], usize) {
        let mut moves = [0u8; N];
        let mut m = 0;
        for c in 0..N {
            if nd.cnt[c] > 0 && (nd.cnt[c] as usize) < n {
                moves[m] = c as u8;
                m += 1;
            }
        }
        moves[..m].sort_unstable_by_key(|&c| nd.cnt[c as usize]);
        (moves, m)
    }

    fn key(&self, nd: &Node, p: i32) -> (u64, i32) {
        (self.sym.canon(nd.dmask), if self.pass { p } else { 0 })
    }

    /// Plain win/loss (true = the player to move wins).
    fn value(&mut self, d: usize, len: usize, n: usize, bmask: u64, wmask: u64, off: i32) -> Result<bool, Abort> {
        if n <= 1 {
            return Ok(false);
        }
        let nd = self.node(d, len, n, bmask, wmask);
        if nd.has1 {
            return Ok(true);
        }
        let p = off + nd.dmask.count_ones() as i32;
        let key = self.key(&nd, p);
        if let Some(&v) = self.val.get(&key) {
            return Ok(v);
        }
        self.vnodes += 1;
        if self.vnodes & 1023 == 0 && now_ms() > self.deadline {
            return Err(Abort);
        }
        let mut res = false;
        if self.pass && p > 0 && !self.value(d, len, n, bmask, wmask, off - 1)? {
            res = true;
        }
        if !res {
            let (moves, m) = Self::moves(&nd, n);
            let off2 = p - nd.dmask.count_ones() as i32 - 1;
            for &c in &moves[..m] {
                let c = c as usize;
                if nd.cnt[c] == 2 {
                    continue;
                }
                let kl = self.kid(d, len, c);
                if !self.value(d + 1, kl, nd.cnt[c] as usize, nd.dmask, nd.xmask, off2)? {
                    res = true;
                    break;
                }
            }
        }
        self.val.insert(key, res);
        Ok(res)
    }

    /// `off` = passes left minus |D| (the node adds its own |D|); ignored for the normal rule.
    fn win_within(&mut self, d: usize, len: usize, n: usize, bmask: u64, wmask: u64, off: i32, k: u8) -> Result<bool, Abort> {
        if n <= 1 || k == 0 {
            return Ok(false);
        }
        let nd = self.node(d, len, n, bmask, wmask);
        if nd.has1 {
            return Ok(true);
        }
        if k < 3 {
            return Ok(false);
        }
        let p = off + nd.dmask.count_ones() as i32;
        let key = self.key(&nd, p);
        let e = *self.tt.get(&key).unwrap_or(&[0, NONE, 0, NONE]);
        if e[1] <= k {
            return Ok(true);
        }
        if e[0] >= k {
            return Ok(false);
        }
        if !self.value(d, len, n, bmask, wmask, off)? {
            return Ok(false);
        }
        self.tick()?;
        let mut res = false;
        if self.pass && p > 0 && !self.value(d, len, n, bmask, wmask, off - 1)? && self.lose_within(d, len, n, bmask, wmask, off - 1, k - 1)? {
            res = true;
        }
        if !res {
            let (moves, m) = Self::moves(&nd, n);
            let off2 = p - nd.dmask.count_ones() as i32 - 1;
            for &c in &moves[..m] {
                let c = c as usize;
                if nd.cnt[c] == 2 {
                    continue; // the opponent wins at once
                }
                let kl = self.kid(d, len, c);
                if self.value(d + 1, kl, nd.cnt[c] as usize, nd.dmask, nd.xmask, off2)? {
                    continue; // not a winning move
                }
                let kl = self.kid(d, len, c);
                if self.lose_within(d + 1, kl, nd.cnt[c] as usize, nd.dmask, nd.xmask, off2, k - 1)? {
                    res = true;
                    break;
                }
            }
        }
        let e = self.tt.entry(key).or_insert([0, NONE, 0, NONE]);
        if res {
            e[1] = e[1].min(k);
        } else {
            e[0] = e[0].max(k);
        }
        Ok(res)
    }

    fn lose_within(&mut self, d: usize, len: usize, n: usize, bmask: u64, wmask: u64, off: i32, j: u8) -> Result<bool, Abort> {
        if n <= 1 {
            return Ok(true);
        }
        let nd = self.node(d, len, n, bmask, wmask);
        if nd.has1 || j < 2 {
            return Ok(false);
        }
        let p = off + nd.dmask.count_ones() as i32;
        let key = self.key(&nd, p);
        let e = *self.tt.get(&key).unwrap_or(&[0, NONE, 0, NONE]);
        if e[3] <= j {
            return Ok(true);
        }
        if e[2] >= j {
            return Ok(false);
        }
        if self.value(d, len, n, bmask, wmask, off)? {
            return Ok(false);
        }
        self.tick()?;
        let mut res = true;
        if self.pass && p > 0 && !self.win_within(d, len, n, bmask, wmask, off - 1, j - 1)? {
            res = false;
        }
        if res {
            let (mut moves, m) = Self::moves(&nd, n);
            // a size-2 child loses at once for us; try the others, largest first (most likely to hold out)
            moves[..m].reverse();
            let off2 = p - nd.dmask.count_ones() as i32 - 1;
            for &c in &moves[..m] {
                let c = c as usize;
                if nd.cnt[c] == 2 {
                    continue; // the opponent wins at once: never an escape
                }
                let kl = self.kid(d, len, c);
                if !self.win_within(d + 1, kl, nd.cnt[c] as usize, nd.dmask, nd.xmask, off2, j - 1)? {
                    res = false;
                    break;
                }
            }
            // if every regular move hands over a size-2 child (and no pass escapes), we lose in 2
        }
        let e = self.tt.entry(key).or_insert([0, NONE, 0, NONE]);
        if res {
            e[3] = e[3].min(j);
        } else {
            e[2] = e[2].max(j);
        }
        Ok(res)
    }

    /// Exact (mover wins, plies to the end) for the position `sols` with `passes` left.
    pub fn length(&mut self, sols: &[u64], passes: i32) -> Result<(bool, u32), Abort> {
        let n = sols.len();
        if n <= 1 {
            return Ok((false, 0));
        }
        self.prepare(sols);
        let w = self.w;
        let d0 = sols.iter().fold(!0u64, |a, &x| a & x) & ((1u64 << N) - 1);
        let off = passes - d0.count_ones() as i32;
        let win = self.value(0, w, n, 0, 0, off)?;
        for k in (if win { 1 } else { 2 }..=250u8).step_by(2) {
            let hit = if k % 2 == 1 { self.win_within(0, w, n, 0, 0, off, k)? } else { self.lose_within(0, w, n, 0, 0, off, k)? };
            if hit {
                return Ok((k % 2 == 1, k as u32));
            }
        }
        unreachable!("game longer than 250 plies")
    }

    /// Does the player to move in `sols` lose within j plies?
    pub fn loses_within(&mut self, sols: &[u64], passes: i32, j: u32) -> Result<bool, Abort> {
        let n = sols.len();
        if n <= 1 {
            return Ok(true);
        }
        self.prepare(sols);
        let w = self.w;
        let d0 = sols.iter().fold(!0u64, |a, &x| a & x) & ((1u64 << N) - 1);
        self.lose_within(0, w, n, 0, 0, passes - d0.count_ones() as i32, j as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{list_counts, Rng, Universe};

    fn dm(v: &[u64]) -> u64 {
        v.iter().fold(!0u64, |a, &x| a & x) & ((1u64 << N) - 1)
    }

    /// plain minimax over explicit lists: (wins, plies); memo on the exact
    /// determined-black mask (no symmetry, no bounds)
    fn naive(v: &[u64], p: i32, pass: bool, memo: &mut HashMap<(u64, i32), (bool, u32)>) -> (bool, u32) {
        let n = v.len();
        if n <= 1 {
            return (false, 0);
        }
        if let Some(&r) = memo.get(&(dm(v), p)) {
            return r;
        }
        let cnt = list_counts(v);
        let d = dm(v);
        let mut kids: Vec<(bool, u32)> = Vec::new();
        if pass && p > 0 {
            kids.push(naive(v, p - 1, pass, memo));
        }
        for c in 0..N {
            if cnt[c] == 0 || cnt[c] as usize == n {
                continue;
            }
            let kid: Vec<u64> = v.iter().copied().filter(|x| (x >> c) & 1 == 1).collect();
            let p2 = p + dm(&kid).count_ones() as i32 - d.count_ones() as i32 - 1;
            kids.push(naive(&kid, p2, pass, memo));
        }
        let wins: Vec<u32> = kids.iter().filter(|k| !k.0).map(|k| k.1 + 1).collect();
        let r = if let Some(&m) = wins.iter().min() {
            (true, m)
        } else {
            (false, kids.iter().map(|k| k.1 + 1).max().unwrap())
        };
        memo.insert((dm(v), p), r);
        r
    }

    #[test]
    fn length_matches_plain_minimax() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../play/data/solutions.bin");
        let u = Universe::from_le_bytes(&std::fs::read(path).unwrap());
        let mut rng = Rng::new(5);
        let mut checked = [0, 0];
        while checked[0] + checked[1] < 160 {
            let mut st: Vec<u8> = Vec::new();
            let mut list = u.to_list(&u.root());
            while list.len() > 400 {
                let cnt = list_counts(&list);
                let lg = crate::legal(&cnt, list.len());
                let c = lg[rng.below(lg.len())];
                st.push(c);
                list.retain(|x| (x >> c) & 1 == 1);
            }
            if list.len() < 20 {
                continue;
            }
            for pass in [false, true] {
                let p = dm(&list).count_ones() as i32 - st.len() as i32;
                let mut ds = Dist::new(pass);
                assert_eq!(ds.length(&list, p).ok().unwrap(), naive(&list, p, pass, &mut HashMap::new()), "stones {st:?} pass {pass}");
                checked[pass as usize] += 1;
            }
        }
    }
}
