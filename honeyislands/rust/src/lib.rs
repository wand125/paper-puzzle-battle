//! Honey Islands battle engine (native and wasm32).
//!
//! Board: 61 hexagonal cells (radius 4). A solution is the set of 25 black
//! cells that splits the 36 white cells into six islands of six; all
//! 1,304,983 solutions are given as input (bit c of a u64 = cell c is black).
//!
//! Battle rule: players alternately place a black stone on a cell that is
//! still undetermined (black in some remaining solutions, white in others).
//! Whoever leaves exactly one solution wins; the opponent has no move left.

pub mod wasm;
pub mod dist;

pub const N: usize = 61;

// ---- clock -----------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "env")]
extern "C" {
    fn hib_now() -> f64;
}
#[cfg(target_arch = "wasm32")]
pub fn now_ms() -> f64 {
    unsafe { hib_now() }
}
#[cfg(not(target_arch = "wasm32"))]
pub fn now_ms() -> f64 {
    use std::sync::OnceLock;
    use std::time::Instant;
    static T0: OnceLock<Instant> = OnceLock::new();
    T0.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0
}

// ---- rng ---------------------------------------------------------------------

pub struct Rng(u64);
impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn below(&mut self, n: usize) -> usize {
        (((self.next() >> 32) * n as u64) >> 32) as usize
    }
    pub fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

// ---- solution sets ---------------------------------------------------------

/// A set of remaining solutions: an explicit list (small sets) or a sparse
/// bitset over global solution indices holding only non-zero 64-bit words.
#[derive(Clone)]
pub enum Set {
    List(Vec<u64>),
    Bits { idx: Vec<u32>, val: Vec<u64>, n: u32 },
}

impl Set {
    pub fn n(&self) -> usize {
        match self {
            Set::List(v) => v.len(),
            Set::Bits { n, .. } => *n as usize,
        }
    }
}

pub const LIST_LIMIT: usize = 20000;

pub fn list_counts(v: &[u64]) -> [u32; N] {
    let mut o = [0u32; N];
    for &s in v {
        let mut a = s;
        while a != 0 {
            o[a.trailing_zeros() as usize] += 1;
            a &= a - 1;
        }
    }
    o
}

/// All solutions plus one bit column per cell (bit i of column c = solution i
/// has c black), so filtering a bitset is a word-wise AND.
pub struct Universe {
    pub sols: Vec<u64>,
    w: usize,
    col: Vec<u64>,
}

impl Universe {
    pub fn new(sols: Vec<u64>) -> Self {
        let n = sols.len();
        let w = n.div_ceil(64);
        let mut col = vec![0u64; N * w];
        for (i, &s) in sols.iter().enumerate() {
            let mut a = s;
            while a != 0 {
                let c = a.trailing_zeros() as usize;
                col[c * w + i / 64] |= 1u64 << (i % 64);
                a &= a - 1;
            }
        }
        Universe { sols, w, col }
    }

    /// Little-endian pairs (lo = cells 0..30, hi = cells 31..60).
    pub fn from_le_bytes(b: &[u8]) -> Self {
        let sols = b
            .chunks_exact(8)
            .map(|c| {
                let lo = u32::from_le_bytes([c[0], c[1], c[2], c[3]]) as u64;
                let hi = u32::from_le_bytes([c[4], c[5], c[6], c[7]]) as u64;
                lo | (hi << 31)
            })
            .collect();
        Universe::new(sols)
    }

    pub fn root(&self) -> Set {
        let n = self.sols.len();
        let mut val = vec![!0u64; self.w];
        if n % 64 != 0 {
            val[self.w - 1] = (1u64 << (n % 64)) - 1;
        }
        Set::Bits { idx: (0..self.w as u32).collect(), val, n: n as u32 }
    }

    pub fn count(&self, s: &Set, c: usize) -> usize {
        match s {
            Set::List(v) => v.iter().filter(|&&x| (x >> c) & 1 == 1).count(),
            Set::Bits { idx, val, .. } => {
                let col = &self.col[c * self.w..(c + 1) * self.w];
                idx.iter().zip(val).map(|(&i, &v)| (v & col[i as usize]).count_ones() as usize).sum()
            }
        }
    }

    pub fn counts(&self, s: &Set) -> [u32; N] {
        match s {
            Set::List(v) => list_counts(v),
            Set::Bits { .. } => {
                let mut o = [0u32; N];
                for (c, x) in o.iter_mut().enumerate() {
                    *x = self.count(s, c) as u32;
                }
                o
            }
        }
    }

    pub fn child(&self, s: &Set, c: usize) -> Set {
        match s {
            Set::List(v) => Set::List(v.iter().copied().filter(|x| (x >> c) & 1 == 1).collect()),
            Set::Bits { idx, val, .. } => {
                let col = &self.col[c * self.w..(c + 1) * self.w];
                let mut ni = Vec::with_capacity(idx.len());
                let mut nv = Vec::with_capacity(idx.len());
                let mut n = 0u32;
                for (&i, &v) in idx.iter().zip(val) {
                    let x = v & col[i as usize];
                    if x != 0 {
                        ni.push(i);
                        nv.push(x);
                        n += x.count_ones();
                    }
                }
                let out = Set::Bits { idx: ni, val: nv, n };
                if n as usize <= LIST_LIMIT {
                    Set::List(self.to_list(&out))
                } else {
                    out
                }
            }
        }
    }

    pub fn to_list(&self, s: &Set) -> Vec<u64> {
        match s {
            Set::List(v) => v.clone(),
            Set::Bits { idx, val, n } => {
                let mut out = Vec::with_capacity(*n as usize);
                for (&i, &v) in idx.iter().zip(val) {
                    let mut x = v;
                    while x != 0 {
                        out.push(self.sols[i as usize * 64 + x.trailing_zeros() as usize]);
                        x &= x - 1;
                    }
                }
                out
            }
        }
    }

    pub fn play(&self, stones: &[u8]) -> Set {
        let mut s = self.root();
        for &c in stones {
            s = self.child(&s, c as usize);
        }
        s
    }
}

/// Undetermined cells of a set with the given counts.
pub fn legal(cnt: &[u32; N], n: usize) -> Vec<u8> {
    (0..N).filter(|&c| cnt[c] > 0 && (cnt[c] as usize) < n).map(|c| c as u8).collect()
}

// ---- board symmetries ------------------------------------------------------
//
// The 12 symmetries of the hexagon (6 rotations x reflection) map solutions
// to solutions, so positions related by a symmetry have the same value.

pub fn cells() -> Vec<(i32, i32)> {
    let mut v = Vec::new();
    for y in -4..=4 {
        for x in -4..=4 {
            if (x + y as i32).abs() <= 4 {
                v.push((x, y));
            }
        }
    }
    v
}

pub struct Sym {
    /// table[k][byte][value] = image of that byte's bits under symmetry k
    table: Vec<[[u64; 256]; 8]>,
    pub perm: Vec<[u8; N]>,
}

impl Sym {
    pub fn new() -> Self {
        let cs = cells();
        let index = |p: (i32, i32)| cs.iter().position(|&q| q == p).unwrap() as u8;
        let rot = |(x, y): (i32, i32)| (x + y, -x); // 60 degrees in axial coordinates
        let mut perm = Vec::new();
        for refl in [false, true] {
            for r in 0..6 {
                let mut p = [0u8; N];
                for (i, &c) in cs.iter().enumerate() {
                    let mut d = if refl { (c.1, c.0) } else { c };
                    for _ in 0..r {
                        d = rot(d);
                    }
                    p[i] = index(d);
                }
                perm.push(p);
            }
        }
        let table = perm
            .iter()
            .map(|p| {
                let mut t = [[0u64; 256]; 8];
                for (b, row) in t.iter_mut().enumerate() {
                    for (v, out) in row.iter_mut().enumerate() {
                        let mut m = 0u64;
                        for j in 0..8 {
                            let c = b * 8 + j;
                            if c < N && (v >> j) & 1 == 1 {
                                m |= 1u64 << p[c];
                            }
                        }
                        *out = m;
                    }
                }
                t
            })
            .collect();
        Sym { table, perm }
    }

    #[inline]
    pub fn apply(&self, k: usize, m: u64) -> u64 {
        let t = &self.table[k];
        let mut o = 0;
        for (b, row) in t.iter().enumerate() {
            o |= row[((m >> (8 * b)) & 0xff) as usize];
        }
        o
    }

    /// Smallest image of the mask over all 12 symmetries.
    #[inline]
    pub fn canon(&self, m: u64) -> u64 {
        (0..12).map(|k| self.apply(k, m)).min().unwrap()
    }
}

impl Default for Sym {
    fn default() -> Self {
        Sym::new()
    }
}

// ---- exact solver ------------------------------------------------------------
//
// The remaining set equals {s : D ⊆ s} where D is its determined-black mask,
// so D is a canonical transposition key valid across searches.

pub struct Abort;

/// Lock-free transposition table shared by all solver threads of a process.
/// A slot holds (key << 2) | (value << 1) | 1; racing writes may drop an
/// entry but never corrupt one.
pub struct SharedTT {
    slots: Vec<std::sync::atomic::AtomicU64>,
    shift: u32,
}

impl SharedTT {
    pub fn new(bits: u32) -> Self {
        SharedTT { slots: (0..1usize << bits).map(|_| std::sync::atomic::AtomicU64::new(0)).collect(), shift: 64 - bits }
    }
    #[inline]
    fn slot(&self, k: u64) -> usize {
        (k.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> self.shift) as usize
    }
    pub fn get(&self, k: u64) -> Option<bool> {
        use std::sync::atomic::Ordering::Relaxed;
        let m = self.slots.len() - 1;
        let h = self.slot(k);
        for p in 0..4 {
            let e = self.slots[(h + p) & m].load(Relaxed);
            if e == 0 {
                return None;
            }
            if e >> 2 == k {
                return Some((e >> 1) & 1 == 1);
            }
        }
        None
    }
    pub fn put(&self, k: u64, v: bool) {
        use std::sync::atomic::Ordering::Relaxed;
        let m = self.slots.len() - 1;
        let h = self.slot(k);
        let e = (k << 2) | ((v as u64) << 1) | 1;
        for p in 0..4 {
            let s = &self.slots[(h + p) & m];
            let cur = s.load(Relaxed);
            if cur == 0 || cur >> 2 == k {
                s.store(e, Relaxed);
                return;
            }
        }
        self.slots[h].store(e, Relaxed);
    }
}

/// Shared table for the pass rule, keyed by (determined-black mask, passes left).
/// Two words per slot; the meta word carries a hash of the key, so a reader that
/// sees half of a racing write finds a mismatch and treats it as a miss.
pub struct SharedTT2 {
    keys: Vec<std::sync::atomic::AtomicU64>,
    metas: Vec<std::sync::atomic::AtomicU64>,
    shift: u32,
}

impl SharedTT2 {
    pub fn new(bits: u32) -> Self {
        let n = 1usize << bits;
        SharedTT2 {
            keys: (0..n).map(|_| std::sync::atomic::AtomicU64::new(0)).collect(),
            metas: (0..n).map(|_| std::sync::atomic::AtomicU64::new(0)).collect(),
            shift: 64 - bits,
        }
    }
    #[inline]
    fn h(k: u64, p: i32) -> u64 {
        (k ^ (p as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93)).wrapping_mul(0x9E37_79B9_7F4A_7C15)
    }
    #[inline]
    fn meta(k: u64, p: i32, v: bool) -> u64 {
        (Self::h(k, p) & 0xFFFF_FFFF_0000_0000) | ((p as u64 & 0x3FFF_FFFF) << 2) | ((v as u64) << 1) | 1
    }
    pub fn get(&self, k: u64, p: i32) -> Option<bool> {
        use std::sync::atomic::Ordering::Relaxed;
        let m = self.keys.len() - 1;
        let h = (Self::h(k, p) >> self.shift) as usize;
        let want = Self::meta(k, p, false) & !2;
        for i in 0..4 {
            let j = (h + i) & m;
            let me = self.metas[j].load(Relaxed);
            if me == 0 {
                return None;
            }
            if me & !2 == want && self.keys[j].load(Relaxed) == k {
                return Some(me & 2 != 0);
            }
        }
        None
    }
    pub fn put(&self, k: u64, p: i32, v: bool) {
        use std::sync::atomic::Ordering::Relaxed;
        let m = self.keys.len() - 1;
        let h = (Self::h(k, p) >> self.shift) as usize;
        let want = Self::meta(k, p, false) & !2;
        let mut j = h & m;
        for i in 0..4 {
            let q = (h + i) & m;
            let me = self.metas[q].load(Relaxed);
            if me == 0 || (me & !2 == want && self.keys[q].load(Relaxed) == k) {
                j = q;
                break;
            }
        }
        self.metas[j].store(0, Relaxed);
        self.keys[j].store(k, Relaxed);
        self.metas[j].store(Self::meta(k, p, v), Relaxed);
    }
}

/// Values of large positions shared between processes and machines:
/// imported from a merged file, never evicted; new results are queued for export.
pub struct Known {
    pub map: std::sync::RwLock<std::collections::HashMap<u64, bool>>,
    pub export: std::sync::Mutex<Vec<(u64, bool, u32)>>,
    /// pass rule: (key, passes left) -> value; lines "key:passes value n"
    pub pmap: std::sync::RwLock<std::collections::HashMap<(u64, i32), bool>>,
    pub pexport: std::sync::Mutex<Vec<(u64, i32, bool, u32)>>,
    pub min_n: usize,
}

impl Known {
    pub fn new(min_n: usize) -> Self {
        Known { map: Default::default(), export: Default::default(), pmap: Default::default(), pexport: Default::default(), min_n }
    }
    /// Lines "key value [n]"; returns how many keys were new.
    pub fn load_text(&self, text: &str) -> usize {
        let mut m = self.map.write().unwrap();
        let mut pm = self.pmap.write().unwrap();
        let before = m.len() + pm.len();
        for l in text.lines() {
            let mut f = l.split(' ');
            let (Some(k), Some(v)) = (f.next(), f.next()) else { continue };
            if let Some((a, b)) = k.split_once(':') {
                if let (Ok(a), Ok(b)) = (a.parse::<u64>(), b.parse::<i32>()) {
                    pm.insert((a, b), v == "1");
                }
            } else if let Ok(k) = k.parse::<u64>() {
                m.insert(k, v == "1");
            }
        }
        m.len() + pm.len() - before
    }

    /// Opening-book records of 10 bytes (tools/book_export.py): u64 LE key,
    /// i8 passes left, u8 value. `pass` selects the pass-rule table.
    pub fn load_bin(&self, b: &[u8], pass: bool) -> usize {
        let mut m = self.map.write().unwrap();
        let mut pm = self.pmap.write().unwrap();
        let mut k = 0;
        for r in b.chunks_exact(10) {
            let key = u64::from_le_bytes(r[..8].try_into().unwrap());
            let v = r[9] == 1;
            if pass {
                pm.insert((key, r[8] as i8 as i32), v);
            } else {
                m.insert(key, v);
            }
            k += 1;
        }
        k
    }
}

/// A winning move found in the opening book: a move into a position recorded
/// as lost for the opponent (random among them). None if the book has none.
pub fn book_choose(u: &Universe, stones: &[u8], pass: bool, known: &Known, sym: &Sym, seed: u64) -> Option<usize> {
    let list = u.to_list(&u.play(stones));
    let n = list.len();
    if n <= 1 {
        return None;
    }
    let full = (1u64 << N) - 1;
    let mut cnt = [0u32; N];
    let mut and = [full; N];
    for &s in &list {
        let mut a = s;
        while a != 0 {
            let c = a.trailing_zeros() as usize;
            cnt[c] += 1;
            and[c] &= s;
            a &= a - 1;
        }
    }
    let dm: u64 = (0..N).filter(|&c| cnt[c] as usize == n).fold(0, |m, c| m | 1u64 << c);
    let passes = dm.count_ones() as i32 - stones.len() as i32;
    let map = known.map.read().unwrap();
    let pmap = known.pmap.read().unwrap();
    let lost = |d: u64, p: i32| -> bool {
        let key = sym.canon(d);
        if pass { pmap.get(&(key, p)) == Some(&false) } else { map.get(&key) == Some(&false) }
    };
    let mut wins: Vec<usize> = Vec::new();
    for c in 0..N {
        if cnt[c] == 0 || cnt[c] as usize == n {
            continue;
        }
        if cnt[c] == 1 {
            return Some(c);
        }
        let d = and[c] & full;
        if lost(d, passes + d.count_ones() as i32 - dm.count_ones() as i32 - 1) {
            wins.push(c);
        }
    }
    if pass && passes > 0 && lost(dm, passes - 1) {
        let st = stone_mask(stones);
        wins.extend((0..N).find(|&c| dm >> c & 1 == 1 && st >> c & 1 == 0));
    }
    if wins.is_empty() {
        None
    } else {
        Some(wins[Rng::new(seed).below(wins.len())])
    }
}

const EMPTY: u64 = u64::MAX;

pub struct Solver {
    tt_key: Vec<u64>,
    tt_val: Vec<u8>,
    shift: u32,
    pub nodes: u64,
    pub node_limit: u64,
    pub deadline: f64,
    col: Vec<u64>,
    w: usize,
    ibuf: Vec<u32>,
    vbuf: Vec<u64>,
    sym: Option<Sym>,
    pub shared: Option<std::sync::Arc<SharedTT>>,
    pub known: Option<std::sync::Arc<Known>>,
    /// set from outside to abandon the current search (returns Err(Abort))
    pub stop: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    /// table for the pass variant: (determined-black mask, passes left) -> value
    vtt: std::collections::HashMap<(u64, i32), bool>,
    pub vshared: Option<std::sync::Arc<SharedTT2>>,
    /// search shape: [won via a size-1 child, won in the move loop, moves tried in those,
    /// lost nodes, moves refuted in those, pass tries]
    pub shape: [u64; 6],
}

impl Solver {
    /// Transposition keys are reduced modulo the board symmetries.
    pub fn with_symmetry(mut self) -> Self {
        self.sym = Some(Sym::new());
        self
    }

    pub fn new(bits: u32) -> Self {
        let size = 1usize << bits;
        Solver {
            tt_key: vec![EMPTY; size],
            tt_val: vec![0; size],
            shift: 64 - bits,
            nodes: 0,
            node_limit: u64::MAX,
            deadline: f64::INFINITY,
            col: Vec::new(),
            w: 0,
            ibuf: Vec::new(),
            vbuf: Vec::new(),
            sym: None,
            shared: None,
            known: None,
            stop: None,
            vtt: Default::default(),
            vshared: None,
            shape: [0; 6],
        }
    }

    pub fn clear(&mut self) {
        self.tt_key.fill(EMPTY);
    }

    #[inline]
    fn slot(&self, k: u64) -> usize {
        (k.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> self.shift) as usize
    }

    fn tt_get(&self, k: u64) -> Option<bool> {
        if let Some(t) = &self.shared {
            return t.get(k);
        }
        let m = self.tt_key.len() - 1;
        let h = self.slot(k);
        for p in 0..4 {
            let j = (h + p) & m;
            let kk = self.tt_key[j];
            if kk == k {
                return Some(self.tt_val[j] != 0);
            }
            if kk == EMPTY {
                return None;
            }
        }
        None
    }

    fn tt_put(&mut self, k: u64, v: bool) {
        if let Some(t) = &self.shared {
            t.put(k, v);
            return;
        }
        let m = self.tt_key.len() - 1;
        let h = self.slot(k);
        let mut j = h;
        for p in 0..4 {
            let q = (h + p) & m;
            if self.tt_key[q] == EMPTY || self.tt_key[q] == k {
                j = q;
                break;
            }
        }
        self.tt_key[j] = k;
        self.tt_val[j] = v as u8;
    }

    /// true = the player to move wins. Works on bitsets over a local
    /// re-indexing of `sols`; scratch buffers are one slice per depth.
    pub fn win(&mut self, sols: &[u64]) -> Result<bool, Abort> {
        let n = sols.len();
        if n <= 1 {
            return Ok(false);
        }
        let w = n.div_ceil(64);
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
        let depth_max = 64;
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
        self.win_b(0, w, n, 0, 0)
    }

    fn win_b(&mut self, d: usize, len: usize, n: usize, bmask: u64, wmask: u64) -> Result<bool, Abort> {
        if n <= 1 {
            return Ok(false);
        }
        let w = self.w;
        let mut cnt = [0u32; N];
        let mut dmask = bmask;
        let mut xmask = wmask;
        let mut res = false;
        {
            let idx = &self.ibuf[d * w..d * w + len];
            let val = &self.vbuf[d * w..d * w + len];
            for c in 0..N {
                let bit = 1u64 << c;
                if bmask & bit != 0 {
                    cnt[c] = n as u32;
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
                cnt[c] = k;
                if k as usize == n {
                    dmask |= bit;
                } else if k == 0 {
                    xmask |= bit;
                } else if k == 1 {
                    res = true;
                }
            }
        }
        let key = match &self.sym {
            Some(sym) => sym.canon(dmask),
            None => dmask,
        };
        if let Some(kn) = &self.known {
            if n >= kn.min_n {
                if let Some(&v) = kn.map.read().unwrap().get(&key) {
                    return Ok(v);
                }
            }
        }
        if let Some(v) = self.tt_get(key) {
            return Ok(v);
        }
        self.nodes += 1;
        if self.nodes & 1023 == 0
            && (self.nodes > self.node_limit
                || now_ms() > self.deadline
                || self.stop.as_ref().is_some_and(|f| f.load(std::sync::atomic::Ordering::Relaxed)))
        {
            return Err(Abort);
        }
        if !res {
            // a child of size 2 hands the opponent an immediate win
            let mut moves = [0u8; N];
            let mut m = 0;
            for c in 0..N {
                if cnt[c] > 2 && (cnt[c] as usize) < n {
                    moves[m] = c as u8;
                    m += 1;
                }
            }
            // small children first: cheap to refute, and often decisive
            moves[..m].sort_unstable_by_key(|&c| cnt[c as usize]);
            let mut tried = 0u64;
            for &c in &moves[..m] {
                let c = c as usize;
                tried += 1;
                let k;
                {
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
                    k = kk;
                }
                if !self.win_b(d + 1, k, cnt[c] as usize, dmask, xmask)? {
                    res = true;
                    break;
                }
            }
            if res { self.shape[1] += 1; self.shape[2] += tried; } else { self.shape[3] += 1; self.shape[4] += tried; }
        } else {
            self.shape[0] += 1;
        }
        self.tt_put(key, res);
        if let Some(kn) = &self.known {
            if n >= kn.min_n {
                kn.export.lock().unwrap().push((key, res, n as u32));
            }
        }
        Ok(res)
    }

    /// Pass variant: a stone may also go on a cell already determined black,
    /// which leaves the solution set unchanged. `passes` = such cells still free
    /// (determined-black cells minus stones placed). true = player to move wins.
    pub fn win_pass(&mut self, sols: &[u64], passes: i32) -> Result<bool, Abort> {
        let n = sols.len();
        if n <= 1 {
            return Ok(false);
        }
        if self.vtt.len() > 4_000_000 {
            self.vtt.clear();
        }
        let d_top = sols.iter().fold(!0u64, |a, &x| a & x) & ((1u64 << N) - 1);
        self.prepare(sols);
        let w = self.w;
        self.win_vb(0, w, n, 0, 0, passes - d_top.count_ones() as i32)
    }

    /// Local bitset columns and root buffers for `sols` (shared by both rules).
    fn prepare(&mut self, sols: &[u64]) {
        let n = sols.len();
        let w = n.div_ceil(64);
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
        self.ibuf.clear();
        self.ibuf.resize(64 * w, 0);
        self.vbuf.clear();
        self.vbuf.resize(64 * w, 0);
        for j in 0..w {
            self.ibuf[j] = j as u32;
            self.vbuf[j] = !0;
        }
        if n % 64 != 0 {
            self.vbuf[w - 1] = (1u64 << (n % 64)) - 1;
        }
    }

    // `offset` + |determined black| = passes left at this node.
    fn win_vb(&mut self, d: usize, len: usize, n: usize, bmask: u64, wmask: u64, offset: i32) -> Result<bool, Abort> {
        if n <= 1 {
            return Ok(false);
        }
        let w = self.w;
        let mut cnt = [0u32; N];
        let mut dmask = bmask;
        let mut xmask = wmask;
        let mut res = false;
        {
            let idx = &self.ibuf[d * w..d * w + len];
            let val = &self.vbuf[d * w..d * w + len];
            for c in 0..N {
                let bit = 1u64 << c;
                if bmask & bit != 0 {
                    cnt[c] = n as u32;
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
                cnt[c] = k;
                if k as usize == n {
                    dmask |= bit;
                } else if k == 0 {
                    xmask |= bit;
                } else if k == 1 {
                    res = true;
                }
            }
        }
        let passes = offset + dmask.count_ones() as i32;
        if res {
            return Ok(true);
        }
        let key = match &self.sym {
            Some(sym) => sym.canon(dmask),
            None => dmask,
        };
        if let Some(kn) = &self.known {
            if n >= kn.min_n {
                if let Some(&v) = kn.pmap.read().unwrap().get(&(key, passes)) {
                    return Ok(v);
                }
            }
        }
        let hit = match &self.vshared {
            Some(t) => t.get(key, passes),
            None => self.vtt.get(&(key, passes)).copied(),
        };
        if let Some(v) = hit {
            return Ok(v);
        }
        self.nodes += 1;
        if self.nodes & 1023 == 0
            && (self.nodes > self.node_limit
                || now_ms() > self.deadline
                || self.stop.as_ref().is_some_and(|f| f.load(std::sync::atomic::Ordering::Relaxed)))
        {
            return Err(Abort);
        }
        // a pass hands the same set to the opponent with one pass fewer
        if passes > 0 {
            self.shape[5] += 1;
            if !self.win_vb(d, len, n, dmask, xmask, offset - 1)? {
                res = true;
            }
        }
        if !res {
            let mut moves = [0u8; N];
            let mut m = 0;
            for c in 0..N {
                if cnt[c] > 2 && (cnt[c] as usize) < n {
                    moves[m] = c as u8;
                    m += 1;
                }
            }
            moves[..m].sort_unstable_by_key(|&c| cnt[c as usize]);
            let mut tried = 0u64;
            for &c in &moves[..m] {
                let c = c as usize;
                tried += 1;
                let k;
                {
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
                    k = kk;
                }
                // child passes = passes + (|D'| - |D|) - 1
                let off = passes - dmask.count_ones() as i32 - 1;
                if !self.win_vb(d + 1, k, cnt[c] as usize, dmask, xmask, off)? {
                    res = true;
                    break;
                }
            }
            if res { self.shape[1] += 1; self.shape[2] += tried; } else { self.shape[3] += 1; self.shape[4] += tried; }
        } else {
            self.shape[0] += 1;
        }
        match &self.vshared {
            Some(t) => t.put(key, passes, res),
            None => {
                self.vtt.insert((key, passes), res);
            }
        }
        if let Some(kn) = &self.known {
            if n >= kn.min_n {
                kn.pexport.lock().unwrap().push((key, passes, res, n as u32));
            }
        }
        Ok(res)
    }

    /// Pass rule: a winning cell (a pass is a free determined-black cell), or None.
    pub fn best_move_pass(&mut self, sols: &[u64], passes: i32, stones: &[u8]) -> Result<Option<usize>, Abort> {
        let n = sols.len();
        let cnt = list_counts(sols);
        if let Some(c) = (0..N).find(|&c| cnt[c] == 1) {
            return Ok(Some(c));
        }
        let dm: u64 = (0..N).filter(|&c| cnt[c] as usize == n).fold(0, |m, c| m | 1u64 << c);
        let st = stones.iter().fold(0u64, |m, &c| m | 1u64 << c);
        if passes > 0 && !self.win_pass(sols, passes - 1)? {
            return Ok((0..N).find(|&c| dm >> c & 1 == 1 && st >> c & 1 == 0));
        }
        let mut moves: Vec<usize> = (0..N).filter(|&c| cnt[c] > 2 && (cnt[c] as usize) < n).collect();
        moves.sort_unstable_by_key(|&c| cnt[c]);
        for c in moves {
            let kid: Vec<u64> = sols.iter().copied().filter(|x| (x >> c) & 1 == 1).collect();
            let kd = kid.iter().fold(!0u64, |a, &x| a & x) & ((1u64 << N) - 1);
            let p2 = passes + kd.count_ones() as i32 - dm.count_ones() as i32 - 1;
            if !self.win_pass(&kid, p2)? {
                return Ok(Some(c));
            }
        }
        Ok(None)
    }

    /// A winning move, or None if every move loses.
    pub fn best_move(&mut self, sols: &[u64]) -> Result<Option<usize>, Abort> {
        let cnt = list_counts(sols);
        let mut moves = legal(&cnt, sols.len());
        moves.sort_unstable_by_key(|&c| cnt[c as usize]);
        for &c in &moves {
            if cnt[c as usize] == 1 {
                return Ok(Some(c as usize));
            }
        }
        for &c in &moves {
            let kid: Vec<u64> = sols.iter().copied().filter(|x| (x >> c) & 1 == 1).collect();
            if !self.win(&kid)? {
                return Ok(Some(c as usize));
            }
        }
        Ok(None)
    }
}

// ---- Monte Carlo tree search -------------------------------------------------
//
// Nodes are shared between move orders (keyed by the stone set).
// proven: +1 player to move wins, -1 loses, 0 open.
// v / w: visits / wins for the player who moved INTO the node.

#[derive(Clone)]
pub struct Params {
    pub c: f64,
    pub smart_playout: bool,
    /// at or below this many solutions the root first tries exact proofs
    pub handoff: usize,
    /// children up to this size are proven at the root
    pub proof_limit: usize,
    /// share of the time budget for root proofs
    pub proof_share: f64,
    /// in-tree proofs for nodes up to this size (0 = off)
    pub exact_limit: usize,
    pub solver_nodes: u64,
}

impl Default for Params {
    fn default() -> Self {
        Params { c: 1.4, smart_playout: false, handoff: usize::MAX, proof_limit: 6000, proof_share: 0.9, exact_limit: 0, solver_nodes: 2000 }
    }
}

struct Node {
    n: u32,
    v: u32,
    w: f64,
    proven: i8,
    expanded: bool,
    next_exact: u32,
    moves: Vec<u8>,
    cnt: Vec<u32>,
    kids: Vec<u32>,
}

const NONE: u32 = u32::MAX;

pub struct SearchResult {
    pub mv: i32,
    pub proven: bool,
    pub value: f64,
    pub iters: u64,
    pub playouts: u64,
    pub ms: f64,
}

pub struct Mcts {
    pub p: Params,
    nodes: Vec<Node>,
    map: std::collections::HashMap<u64, u32>,
    pub solver: Solver,
    rng: Rng,
    end: f64,
    pub playouts: u64,
}

fn stone_mask(stones: &[u8]) -> u64 {
    stones.iter().fold(0u64, |m, &c| m | 1u64 << c)
}

impl Mcts {
    pub fn new(p: Params, seed: u64, tt_bits: u32) -> Self {
        Mcts { p, nodes: Vec::new(), map: Default::default(), solver: Solver::new(tt_bits), rng: Rng::new(seed), end: 0.0, playouts: 0 }
    }

    pub fn reseed(&mut self, seed: u64) {
        self.rng = Rng::new(seed);
    }

    fn node(&mut self, st: u64, n: usize) -> u32 {
        if let Some(&i) = self.map.get(&st) {
            return i;
        }
        let i = self.nodes.len() as u32;
        self.nodes.push(Node {
            n: n as u32,
            v: 0,
            w: 0.0,
            proven: if n <= 1 { -1 } else { 0 },
            expanded: false,
            next_exact: 0,
            moves: Vec::new(),
            cnt: Vec::new(),
            kids: Vec::new(),
        });
        self.map.insert(st, i);
        i
    }

    fn expand(&mut self, u: &Universe, nd: u32, s: &Set) {
        let cnt = u.counts(s);
        let moves = legal(&cnt, s.n());
        let node = &mut self.nodes[nd as usize];
        node.cnt = moves.iter().map(|&c| cnt[c as usize]).collect();
        node.kids = vec![NONE; moves.len()];
        node.moves = moves;
        node.expanded = true;
        if node.cnt.contains(&1) {
            node.proven = 1;
        }
    }

    fn kid(&mut self, nd: u32, i: usize, st: u64) -> u32 {
        let k = self.nodes[nd as usize].kids[i];
        if k != NONE {
            return k;
        }
        let c = self.nodes[nd as usize].moves[i];
        let n = self.nodes[nd as usize].cnt[i] as usize;
        let k = self.node(st | 1u64 << c, n);
        self.nodes[nd as usize].kids[i] = k;
        k
    }

    fn settle(&mut self, nd: u32) -> i8 {
        let node = &self.nodes[nd as usize];
        if node.proven != 0 || !node.expanded {
            return node.proven;
        }
        let mut all_win = true;
        let mut res = 0;
        for &k in &node.kids {
            if k != NONE && self.nodes[k as usize].proven == -1 {
                res = 1;
                break;
            }
            if k == NONE || self.nodes[k as usize].proven != 1 {
                all_win = false;
            }
        }
        if res == 0 && all_win {
            res = -1;
        }
        self.nodes[nd as usize].proven = res;
        res
    }

    fn select(&mut self, nd: u32) -> Option<usize> {
        let node = &self.nodes[nd as usize];
        let ln = ((node.v + 1) as f64).ln();
        let mut best = None;
        let mut best_u = f64::NEG_INFINITY;
        for (i, &k) in node.kids.iter().enumerate() {
            let u;
            if k != NONE {
                let kn = &self.nodes[k as usize];
                if kn.proven == 1 {
                    continue;
                }
                if kn.proven == -1 {
                    return Some(i);
                }
                if kn.v == 0 {
                    u = 1e9 + self.rng.unit();
                } else {
                    u = kn.w / kn.v as f64 + self.p.c * (ln / kn.v as f64).sqrt();
                }
            } else {
                u = 1e9 + self.rng.unit();
            }
            if u > best_u {
                best_u = u;
                best = Some(i);
            }
        }
        best
    }

    fn try_exact(&mut self, u: &Universe, nd: u32, s: &Set) {
        let node = &self.nodes[nd as usize];
        if node.proven != 0 || node.n as usize > self.p.exact_limit || node.v < node.next_exact {
            return;
        }
        let v = node.v.max(1);
        self.nodes[nd as usize].next_exact = v * 2;
        let list = match s {
            Set::List(l) => std::borrow::Cow::Borrowed(l),
            _ => std::borrow::Cow::Owned(u.to_list(s)),
        };
        self.solver.nodes = 0;
        self.solver.node_limit = self.p.solver_nodes * v as u64;
        self.solver.deadline = self.end;
        if let Ok(w) = self.solver.win(&list) {
            self.nodes[nd as usize].proven = if w { 1 } else { -1 };
        }
    }

    /// true if the player to move at `s` wins a (lightly guided) random game.
    pub fn playout(&mut self, u: &Universe, s: &Set) -> bool {
        let mut cur: std::borrow::Cow<Set> = std::borrow::Cow::Borrowed(s);
        let mut turn = 0u32;
        let mut dead = 0u64; // determined cells stay determined further down
        loop {
            let n = cur.n();
            if n <= 1 {
                break;
            }
            if self.p.smart_playout && n <= 256 {
                if let Set::List(v) = cur.as_ref() {
                    // take an immediate win, avoid handing one over
                    let cnt = list_counts(v);
                    let mut ok = [0u8; N];
                    let mut no = 0;
                    let mut poor = [0u8; N];
                    let mut np = 0;
                    let mut win = false;
                    for c in 0..N {
                        let k = cnt[c] as usize;
                        if k == 1 {
                            win = true;
                            break;
                        }
                        if k > 2 && k < n {
                            ok[no] = c as u8;
                            no += 1;
                        } else if k == 2 {
                            poor[np] = c as u8;
                            np += 1;
                        }
                    }
                    turn ^= 1;
                    if win {
                        break;
                    }
                    let c = if no > 0 { ok[self.rng.below(no)] } else { poor[self.rng.below(np)] } as usize;
                    cur = std::borrow::Cow::Owned(Set::List(v.iter().copied().filter(|x| (x >> c) & 1 == 1).collect()));
                    continue;
                }
            }
            let c = loop {
                let c = self.rng.below(N);
                if (dead >> c) & 1 == 1 {
                    continue;
                }
                let k = u.count(&cur, c);
                if k > 0 && k < n {
                    break c;
                }
                dead |= 1u64 << c;
            };
            cur = std::borrow::Cow::Owned(u.child(&cur, c));
            dead |= 1u64 << c;
            turn ^= 1;
        }
        self.playouts += 1;
        turn == 1
    }

    fn iterate(&mut self, u: &Universe, root: u32, root_st: u64, root_set: &Set) {
        let mut path = vec![root];
        let mut nd = root;
        let mut st = root_st;
        let mut owned: Option<Set> = None;
        loop {
            let cur = owned.as_ref().unwrap_or(root_set);
            if self.p.exact_limit > 0 {
                self.try_exact(u, nd, cur);
            }
            if self.nodes[nd as usize].proven != 0 {
                break;
            }
            if !self.nodes[nd as usize].expanded {
                self.expand(u, nd, cur);
                break;
            }
            let Some(i) = self.select(nd) else {
                self.settle(nd);
                break;
            };
            let k = self.kid(nd, i, st);
            let c = self.nodes[nd as usize].moves[i] as usize;
            let next = u.child(owned.as_ref().unwrap_or(root_set), c);
            owned = Some(next);
            st |= 1u64 << c;
            path.push(k);
            nd = k;
            if self.nodes[k as usize].v == 0 {
                break;
            }
        }
        let proven = self.nodes[nd as usize].proven;
        let mut r = if proven != 0 {
            (proven == 1) as u32 as f64
        } else {
            let cur = owned.take();
            let s = cur.as_ref().unwrap_or(root_set);
            self.playout(u, s) as u32 as f64
        };
        let last = path.len() - 1;
        for (j, &p) in path.iter().enumerate().rev() {
            let node = &mut self.nodes[p as usize];
            node.v += 1;
            node.w += 1.0 - r;
            r = 1.0 - r;
            if j < last {
                self.settle(p);
            }
        }
    }

    pub fn search(&mut self, u: &Universe, stones: &[u8], ms: f64) -> SearchResult {
        let t0 = now_ms();
        let end = t0 + ms;
        self.end = end;
        if self.nodes.len() > 1_500_000 {
            self.nodes.clear();
            self.map.clear();
        }
        let s = u.play(stones);
        let st = stone_mask(stones);
        let root = self.node(st, s.n());
        let p0 = self.playouts;
        if s.n() <= 1 {
            return SearchResult { mv: -1, proven: true, value: -1.0, iters: 0, playouts: 0, ms: 0.0 };
        }
        if !self.nodes[root as usize].expanded {
            self.expand(u, root, &s);
        }
        // tactical zone: try to prove root children outright, smallest first
        if self.nodes[root as usize].proven == 0 && s.n() <= self.p.handoff {
            let deadline = t0 + ms * self.p.proof_share;
            let node = &self.nodes[root as usize];
            let mut order: Vec<usize> = (0..node.moves.len()).filter(|&i| node.cnt[i] as usize <= self.p.proof_limit).collect();
            order.sort_unstable_by_key(|&i| node.cnt[i]);
            let list = u.to_list(&s);
            for i in order {
                if now_ms() > deadline {
                    break;
                }
                let k = self.kid(root, i, st);
                match self.nodes[k as usize].proven {
                    -1 => break,
                    1 => continue,
                    _ => {}
                }
                let c = self.nodes[root as usize].moves[i];
                let kid: Vec<u64> = list.iter().copied().filter(|x| (x >> c) & 1 == 1).collect();
                self.solver.nodes = 0;
                self.solver.node_limit = u64::MAX;
                self.solver.deadline = deadline;
                match self.solver.win(&kid) {
                    Ok(true) => self.nodes[k as usize].proven = 1,
                    Ok(false) => {
                        self.nodes[k as usize].proven = -1;
                        break;
                    }
                    Err(_) => break,
                }
            }
            self.settle(root);
        }
        let mut iters = 0u64;
        while self.nodes[root as usize].proven == 0 && (now_ms() < end || iters < 8) {
            for _ in 0..8 {
                if self.nodes[root as usize].proven != 0 {
                    break;
                }
                self.iterate(u, root, st, &s);
                iters += 1;
            }
        }
        self.settle(root);
        let node = &self.nodes[root as usize];
        let proven = node.proven;
        let stat = |i: usize| -> (u32, u32, f64, i8) {
            let k = node.kids[i];
            if k == NONE {
                (node.cnt[i], 0, 0.0, 0)
            } else {
                let kn = &self.nodes[k as usize];
                (node.cnt[i], kn.v, kn.w, kn.proven)
            }
        };
        let m = node.moves.len();
        let mut pick: Option<usize> = None;
        if proven == 1 {
            pick = (0..m).find(|&i| stat(i).0 == 1).or_else(|| (0..m).find(|&i| stat(i).3 == -1));
        }
        let mut need_solver = false;
        if pick.is_none() && proven == 1 {
            need_solver = true;
        } else if pick.is_none() && proven == -1 {
            pick = (0..m).max_by_key(|&i| stat(i).0); // every move loses: keep the most room
        } else if pick.is_none() {
            pick = (0..m).filter(|&i| stat(i).3 != 1 && stat(i).1 > 0).max_by_key(|&i| stat(i).1);
            if pick.is_none() {
                pick = (0..m).max_by_key(|&i| stat(i).0);
            }
        }
        let moves = node.moves.clone();
        let value = if proven != 0 {
            proven as f64
        } else {
            let (_, v, w, _) = stat(pick.unwrap());
            if v > 0 { w / v as f64 } else { 0.5 }
        };
        let mv = if need_solver {
            self.solver.node_limit = u64::MAX;
            self.solver.deadline = f64::INFINITY;
            let list = u.to_list(&s);
            match self.solver.best_move(&list) {
                Ok(Some(c)) => c as i32,
                _ => moves[0] as i32,
            }
        } else {
            moves[pick.unwrap()] as i32
        };
        SearchResult { mv, proven: proven != 0, value, iters, playouts: self.playouts - p0, ms: now_ms() - t0 }
    }
}

/// AI for the pass variant. Returns (cell, proven, value). Moves are tried as in the
/// normal AI: prove each option (a pass first, then children smallest first) within
/// most of the budget, then compare the rest with random playouts that also pass.
pub fn pass_choose(u: &Universe, stones: &[u8], ms: f64, seed: u64, solver: &mut Solver) -> (i32, bool, f64) {
    let t0 = now_ms();
    let end = t0 + ms;
    let list = u.to_list(&u.play(stones));
    let n = list.len();
    if n <= 1 {
        return (-1, true, -1.0);
    }
    let cnt = list_counts(&list);
    let dm: u64 = (0..N).filter(|&c| cnt[c] as usize == n).fold(0, |m, c| m | 1u64 << c);
    let stone_mask = stones.iter().fold(0u64, |m, &c| m | 1u64 << c);
    let passes = dm.count_ones() as i32 - stones.len() as i32;
    let pass_cell = (0..N).find(|&c| dm >> c & 1 == 1 && stone_mask >> c & 1 == 0);
    if let Some(c) = (0..N).find(|&c| cnt[c] == 1) {
        return (c as i32, true, 1.0);
    }
    // options: (cell, child solutions, passes left for the opponent)
    let mut opts: Vec<(usize, Vec<u64>, i32)> = Vec::new();
    if passes > 0 {
        if let Some(c) = pass_cell {
            opts.push((c, list.clone(), passes - 1));
        }
    }
    let mut regular: Vec<usize> = (0..N).filter(|&c| cnt[c] > 0 && (cnt[c] as usize) < n).collect();
    regular.sort_unstable_by_key(|&c| cnt[c]);
    for c in regular {
        let kid: Vec<u64> = list.iter().copied().filter(|x| (x >> c) & 1 == 1).collect();
        let kd = kid.iter().fold(!0u64, |a, &x| a & x) & ((1u64 << N) - 1);
        let p2 = passes + kd.count_ones() as i32 - dm.count_ones() as i32 - 1;
        opts.push((c, kid, p2));
    }
    solver.deadline = t0 + ms * 0.9;
    solver.node_limit = u64::MAX;
    let mut lost = vec![false; opts.len()];
    for (i, (c, kid, p2)) in opts.iter().enumerate() {
        if kid.len() > 6000 {
            continue;
        }
        match solver.win_pass(kid, *p2) {
            Ok(false) => return (*c as i32, true, 1.0),
            Ok(true) => lost[i] = true,
            Err(_) => break,
        }
    }
    let open: Vec<usize> = (0..opts.len()).filter(|&i| !lost[i]).collect();
    if open.is_empty() {
        let i = (0..opts.len()).max_by_key(|&i| opts[i].1.len()).unwrap();
        return (opts[i].0 as i32, true, -1.0);
    }
    let mut rng = Rng::new(seed);
    let mut wins = vec![0f64; open.len()];
    let mut vis = vec![0f64; open.len()];
    let mut total = 0f64;
    while now_ms() < end || total < open.len() as f64 {
        let mut best = 0;
        let mut bu = f64::NEG_INFINITY;
        for j in 0..open.len() {
            let uu = if vis[j] == 0.0 { f64::INFINITY } else { wins[j] / vis[j] + (2.0 * (total + 1.0).ln() / vis[j]).sqrt() };
            if uu > bu {
                bu = uu;
                best = j;
            }
        }
        let (_, kid, p2) = &opts[open[best]];
        if !pass_playout(kid, *p2, &mut rng) {
            wins[best] += 1.0;
        }
        vis[best] += 1.0;
        total += 1.0;
    }
    let j = (0..open.len()).max_by(|&a, &b| vis[a].partial_cmp(&vis[b]).unwrap()).unwrap();
    (opts[open[j]].0 as i32, false, wins[j] / vis[j].max(1.0))
}

/// Random game under the pass rule: true if the player to move wins.
fn pass_playout(sols: &[u64], passes: i32, rng: &mut Rng) -> bool {
    let mut cur = sols.to_vec();
    let mut p = passes;
    let mut turn = 0;
    loop {
        let n = cur.len();
        if n <= 1 {
            break;
        }
        let cnt = list_counts(&cur);
        if cnt.contains(&1) {
            turn ^= 1;
            break;
        }
        let dm0 = (0..N).filter(|&c| cnt[c] as usize == n).count() as i32;
        let regular: Vec<usize> = (0..N).filter(|&c| cnt[c] > 0 && (cnt[c] as usize) < n).collect();
        let k = rng.below(regular.len() + p.max(0) as usize);
        if k >= regular.len() {
            p -= 1; // pass
        } else {
            let c = regular[k];
            cur.retain(|x| (x >> c) & 1 == 1);
            let d1 = cur.iter().fold(!0u64, |a, &x| a & x) & ((1u64 << N) - 1);
            p += d1.count_ones() as i32 - dm0 - 1;
        }
        turn ^= 1;
    }
    turn == 1
}

/// Port of the previous JS AI ("flat"): prove small children (smallest first)
/// with the whole budget, then UCB1 over root moves with random playouts.
pub fn flat_choose(u: &Universe, stones: &[u8], ms: f64, rng_seed: u64, solver: &mut Solver) -> i32 {
    let t0 = now_ms();
    let end = t0 + ms;
    let s = u.play(stones);
    let list = u.to_list(&s);
    let cnt = list_counts(&list);
    let mut moves = legal(&cnt, list.len());
    if moves.is_empty() {
        return -1;
    }
    if let Some(&c) = moves.iter().find(|&&c| cnt[c as usize] == 1) {
        return c as i32;
    }
    moves.sort_unstable_by_key(|&c| cnt[c as usize]);
    let mut lost = vec![false; moves.len()];
    solver.deadline = end;
    solver.node_limit = u64::MAX;
    for (i, &c) in moves.iter().enumerate() {
        if cnt[c as usize] > 6000 {
            break;
        }
        let kid: Vec<u64> = list.iter().copied().filter(|x| (x >> c) & 1 == 1).collect();
        match solver.win(&kid) {
            Ok(false) => return c as i32,
            Ok(true) => lost[i] = true,
            Err(_) => break,
        }
    }
    let open: Vec<usize> = (0..moves.len()).filter(|&i| !lost[i]).collect();
    if open.is_empty() {
        return *moves.iter().max_by_key(|&&c| cnt[c as usize]).unwrap() as i32;
    }
    let mut m = Mcts::new(Params { smart_playout: false, ..Default::default() }, rng_seed, 4);
    let kids: Vec<Set> = open.iter().map(|&i| u.child(&s, moves[i] as usize)).collect();
    let mut wins = vec![0f64; open.len()];
    let mut vis = vec![0f64; open.len()];
    let mut total = 0f64;
    while now_ms() < end || total < open.len() as f64 {
        let mut best = 0;
        let mut bu = f64::NEG_INFINITY;
        for j in 0..open.len() {
            let uu = if vis[j] == 0.0 { f64::INFINITY } else { wins[j] / vis[j] + (2.0 * (total + 1.0).ln() / vis[j]).sqrt() };
            if uu > bu {
                bu = uu;
                best = j;
            }
        }
        if !m.playout(u, &kids[best]) {
            wins[best] += 1.0;
        }
        vis[best] += 1.0;
        total += 1.0;
    }
    let j = (0..open.len()).max_by(|&a, &b| vis[a].partial_cmp(&vis[b]).unwrap()).unwrap();
    moves[open[j]] as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symmetries_are_distinct_permutations_preserving_adjacency() {
        let sym = Sym::new();
        let cs = cells();
        let adj = |a: (i32, i32), b: (i32, i32)| {
            let (dx, dy) = (a.0 - b.0, a.1 - b.1);
            matches!((dx, dy), (1, 0) | (-1, 0) | (0, 1) | (0, -1) | (1, -1) | (-1, 1))
        };
        let mut seen = std::collections::HashSet::new();
        for p in &sym.perm {
            let mut img: Vec<u8> = p.to_vec();
            img.sort();
            assert_eq!(img, (0..N as u8).collect::<Vec<_>>());
            for i in 0..N {
                for j in 0..N {
                    assert_eq!(adj(cs[i], cs[j]), adj(cs[p[i] as usize], cs[p[j] as usize]));
                }
            }
            assert!(seen.insert(p.to_vec()));
        }
        assert_eq!(seen.len(), 12);
        let m = 0b1011u64 | 1u64 << 60;
        assert_eq!(sym.canon(m), (0..12).map(|k| sym.apply(k, m)).min().unwrap());
        assert_eq!(sym.apply(0, m), m);
    }
}
