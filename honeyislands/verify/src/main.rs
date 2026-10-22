use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const CELLS: usize = 61;
const ALL: u64 = (1u64 << CELLS) - 1;
const RECORDS: usize = 1_304_983;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Rule {
    Normal,
    Pass,
}
impl Rule {
    fn parse(s: &str) -> Result<Self> {
        match s {
            "normal" => Ok(Self::Normal),
            "pass" => Ok(Self::Pass),
            _ => Err("rule must be normal or pass".into()),
        }
    }
}

fn load_solutions() -> Result<Vec<u64>> {
    // Resolve from this independent crate, not from the caller's working directory.
    // HIBV_DATA overrides the location (for copies of the binary on other machines).
    let path = match std::env::var_os("HIBV_DATA") {
        Some(p) => std::path::PathBuf::from(p),
        None => Path::new(env!("CARGO_MANIFEST_DIR")).join("../analyze/data/solutions.bin"),
    };
    let bytes = fs::read(path)?;
    if bytes.len() != RECORDS * 8 {
        return Err(format!(
            "expected {RECORDS} 8-byte records, got {} bytes",
            bytes.len()
        )
        .into());
    }
    bytes
        .chunks_exact(8)
        .map(|b| {
            let lo = u32::from_le_bytes(b[..4].try_into().unwrap());
            let hi = u32::from_le_bytes(b[4..].try_into().unwrap());
            if lo >> 31 != 0 || hi >> 30 != 0 {
                return Err("out-of-board bit in solution data".into());
            }
            Ok(u64::from(lo) | (u64::from(hi) << 31))
        })
        .collect()
}

fn stones(s: &str) -> Result<(u64, u8)> {
    if s == "-" {
        return Ok((0, 0));
    }
    let mut mask = 0u64;
    for token in s.split(',') {
        let c: usize = token.parse()?;
        if c >= CELLS {
            return Err(format!("cell out of range: {c}").into());
        }
        let bit = 1u64 << c;
        if mask & bit != 0 {
            return Err(format!("duplicate stone: {c}").into());
        }
        mask |= bit;
    }
    Ok((mask, mask.count_ones() as u8))
}
fn select(all: &[u64], mask: u64) -> Vec<u64> {
    all.iter().copied().filter(|s| s & mask == mask).collect()
}
fn determined(s: &[u64]) -> u64 {
    s.iter().fold(ALL, |a, b| a & b)
}

// Sparse local words: empty words are dropped on every intersection.
type Position = Vec<(usize, u64)>;
struct Board {
    solutions: Vec<u64>,
    columns: Vec<Vec<u64>>,
}
impl Board {
    fn new(solutions: Vec<u64>) -> Self {
        let words = (solutions.len() + 63) / 64;
        let mut columns = vec![vec![0u64; words]; CELLS];
        for (i, &s) in solutions.iter().enumerate() {
            let mut bits = s;
            while bits != 0 {
                let c = bits.trailing_zeros() as usize;
                bits &= bits - 1;
                columns[c][i / 64] |= 1u64 << (i % 64);
            }
        }
        Self { solutions, columns }
    }
    fn root(&self) -> Position {
        (0..self.columns[0].len())
            .map(|w| {
                let remaining = self.solutions.len() - w * 64;
                (
                    w,
                    if remaining >= 64 {
                        u64::MAX
                    } else {
                        (1u64 << remaining) - 1
                    },
                )
            })
            .collect()
    }
    fn black(&self, pos: &[(usize, u64)], lower_bound: u64) -> u64 {
        let mut d = ALL;
        for &(w, mut bits) in pos {
            while bits != 0 {
                let b = bits.trailing_zeros() as usize;
                bits &= bits - 1;
                d &= self.solutions[w * 64 + b];
                if d == lower_bound {
                    return d;
                }
            }
        }
        d
    }
    fn child(&self, pos: &[(usize, u64)], c: usize) -> Position {
        pos.iter()
            .filter_map(|&(i, w)| {
                let v = w & self.columns[c][i];
                (v != 0).then_some((i, v))
            })
            .collect()
    }
}

struct Solver {
    rule: Rule,
    tt: HashMap<(u64, u8), bool>,
    cap: usize,
    nodes: u64,
    progress: Arc<AtomicU64>,
}
impl Solver {
    fn new(rule: Rule, cap: usize, progress: Arc<AtomicU64>) -> Self {
        Self {
            rule,
            tt: HashMap::new(),
            cap,
            nodes: 0,
            progress,
        }
    }
    fn solve(&mut self, solutions: Vec<u64>, placed: u8) -> Result<(bool, u64, usize, u8)> {
        if solutions.is_empty() {
            return Err("stones match no solutions".into());
        }
        let n = solutions.len();
        let d = determined(&solutions);
        let p = (d.count_ones() as u8)
            .checked_sub(placed)
            .ok_or("too many stones")?;
        self.nodes = 0;
        self.progress.store(0, Ordering::Relaxed);
        let board = Board::new(solutions);
        let value = self.search(&board, &board.root(), n, d, p);
        self.progress.store(self.nodes, Ordering::Relaxed);
        Ok((value, self.nodes, n, p))
    }
    fn save(&mut self, key: (u64, u8), value: bool) -> bool {
        if self.cap != 0 {
            if self.tt.len() >= self.cap {
                self.tt.clear();
            }
            self.tt.insert(key, value);
        }
        value
    }
    fn search(&mut self, b: &Board, pos: &[(usize, u64)], n: usize, d: u64, p: u8) -> bool {
        // Nodes include internal visits served by the TT, but exclude terminal leaves.
        if n <= 1 {
            return false;
        }
        self.nodes += 1;
        if self.nodes & 1023 == 0 {
            self.progress.store(self.nodes, Ordering::Relaxed);
        }
        let key = (d, if self.rule == Rule::Pass { p } else { 0 });
        if let Some(&value) = self.tt.get(&key) {
            return value;
        }
        let mut moves = [(0usize, 0usize); CELLS];
        let mut len = 0;
        let mut cells = ALL & !d;
        while cells != 0 {
            let c = cells.trailing_zeros() as usize;
            cells &= cells - 1;
            let count: usize = pos
                .iter()
                .map(|&(i, w)| (w & b.columns[c][i]).count_ones() as usize)
                .sum();
            if count == 1 {
                return self.save(key, true);
            }
            // With two distinct solutions, the next player has a singleton move,
            // including under the pass rule. Thus creating size two never wins.
            if count > 2 && count < n {
                moves[len] = (count, c);
                len += 1;
            }
        }
        moves[..len].sort_unstable();
        for &(count, c) in &moves[..len] {
            let child = b.child(pos, c);
            let cd = b.black(&child, d | (1u64 << c));
            let cp = if self.rule == Rule::Pass {
                p + (cd.count_ones() - d.count_ones()) as u8 - 1
            } else {
                0
            };
            if !self.search(b, &child, count, cd, cp) {
                return self.save(key, true);
            }
        }
        if self.rule == Rule::Pass && p > 0 && !self.search(b, pos, n, d, p - 1) {
            return self.save(key, true);
        }
        self.save(key, false)
    }
}

struct Job {
    key: String,
    mask: u64,
    placed: u8,
}
struct Running {
    job: Mutex<Option<(String, Instant)>>,
    nodes: Arc<AtomicU64>,
}
fn run_jobs(
    rule: Rule,
    jobs_path: &str,
    results_path: &str,
    threads: usize,
    cap: usize,
    all: Vec<u64>,
) -> Result<()> {
    if threads == 0 {
        return Err("threads must be positive".into());
    }
    let prior = match fs::read_to_string(results_path) {
        Ok(s) => s,
        Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e.into()),
    };
    let finished: HashSet<String> = prior
        .lines()
        .filter_map(|l| l.split_whitespace().next().map(str::to_owned))
        .collect();
    let mut seen = HashSet::new();
    let mut queue = VecDeque::new();
    let mut skipped = 0;
    for (i, line) in fs::read_to_string(jobs_path)?.lines().enumerate() {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.is_empty() {
            continue;
        }
        if fields.len() != 2 {
            return Err(format!("jobs line {}: expected key stones", i + 1).into());
        }
        if !seen.insert(fields[0].to_owned()) {
            return Err(format!("duplicate job key: {}", fields[0]).into());
        }
        let (mask, placed) = stones(fields[1])?;
        if finished.contains(fields[0]) {
            skipped += 1;
            continue;
        }
        queue.push_back(Job {
            key: fields[0].to_owned(),
            mask,
            placed,
        });
    }
    let total = queue.len() + skipped;
    let worker_count = threads.min(queue.len());
    let queue = Mutex::new(queue);
    let mut output = OpenOptions::new()
        .create(true)
        .append(true)
        .open(results_path)?;
    if !prior.is_empty() && !prior.ends_with('\n') {
        writeln!(output)?;
        output.flush()?;
    }
    let output = Mutex::new(output);
    let done = AtomicUsize::new(skipped);
    let running: Vec<_> = (0..worker_count)
        .map(|_| Running {
            job: Mutex::new(None),
            nodes: Arc::new(AtomicU64::new(0)),
        })
        .collect();
    let stop = (Mutex::new(false), Condvar::new());
    let start = Instant::now();
    eprintln!("start: {skipped}/{total} jobs already done, {worker_count} workers");
    thread::scope(|scope| -> Result<()> {
        let monitor = scope.spawn(|| {
            let (lock, cv) = &stop;
            let mut stopped = lock.lock().unwrap();
            loop {
                let (guard, _) = cv
                    .wait_timeout_while(stopped, Duration::from_secs(30), |v| !*v)
                    .unwrap();
                stopped = guard;
                if *stopped {
                    break;
                }
                eprintln!(
                    "elapsed {:.0}s: {}/{} jobs done",
                    start.elapsed().as_secs_f64(),
                    done.load(Ordering::Relaxed),
                    total
                );
                for slot in &running {
                    if let Some((key, since)) = &*slot.job.lock().unwrap() {
                        eprintln!(
                            "  {key}: {:.0}s, {} nodes",
                            since.elapsed().as_secs_f64(),
                            slot.nodes.load(Ordering::Relaxed)
                        );
                    }
                }
            }
        });
        let mut workers = Vec::new();
        for slot in &running {
            let queue = &queue;
            let output = &output;
            let all = &all;
            let done = &done;
            workers.push(scope.spawn(move || -> Result<()> {
                let mut solver = Solver::new(rule, cap, slot.nodes.clone());
                loop {
                    let job = match queue.lock().unwrap().pop_front() {
                        Some(j) => j,
                        None => break,
                    };
                    let since = Instant::now();
                    slot.nodes.store(0, Ordering::Relaxed);
                    *slot.job.lock().unwrap() = Some((job.key.clone(), since));
                    let result = solver.solve(select(all, job.mask), job.placed);
                    let (value, nodes, n, _) =
                        result.map_err(|e| format!("job {}: {e}", job.key))?;
                    let ms = since.elapsed().as_millis();
                    {
                        let mut file = output.lock().unwrap();
                        writeln!(file, "{} {} {nodes} {ms} {n}", job.key, u8::from(value))?;
                        file.flush()?;
                    }
                    *slot.job.lock().unwrap() = None;
                    done.fetch_add(1, Ordering::Relaxed);
                }
                Ok(())
            }));
        }
        let mut error = None;
        for worker in workers {
            match worker.join() {
                Ok(Ok(())) => (),
                Ok(Err(e)) => {
                    error.get_or_insert(e);
                }
                Err(_) => {
                    error.get_or_insert_with(|| "worker panicked".into());
                }
            }
        }
        *stop.0.lock().unwrap() = true;
        stop.1.notify_all();
        monitor.join().map_err(|_| "progress monitor panicked")?;
        if let Some(e) = error {
            return Err(e);
        }
        Ok(())
    })?;
    eprintln!(
        "finished: {total}/{total} jobs, {:.3}s",
        start.elapsed().as_secs_f64()
    );
    Ok(())
}
fn main() {
    if let Err(e) = cli() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
fn cli() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let usage = "usage: hibverify solve <normal|pass> <jobs file> <results file> <threads>\n       hibverify play <normal|pass> <stones>";
    if args.len() < 2 {
        return Err(usage.into());
    }
    let cap = match std::env::var("HIBV_TT_MAX") {
        Ok(v) => v.parse()?,
        Err(std::env::VarError::NotPresent) => 30_000_000,
        Err(e) => return Err(e.into()),
    };
    match args[1].as_str() {
        "solve" if args.len() == 6 => run_jobs(
            Rule::parse(&args[2])?,
            &args[3],
            &args[4],
            args[5].parse()?,
            cap,
            load_solutions()?,
        ),
        "play" if args.len() == 4 => {
            let rule = Rule::parse(&args[2])?;
            let (mask, placed) = stones(&args[3])?;
            let all = load_solutions()?;
            let start = Instant::now();
            let mut solver = Solver::new(rule, cap, Arc::new(AtomicU64::new(0)));
            let (value, nodes, n, p) = solver.solve(select(&all, mask), placed)?;
            println!(
                "n={n} p={p} value={} nodes={nodes} ms={}",
                u8::from(value),
                start.elapsed().as_millis()
            );
            Ok(())
        }
        _ => Err(usage.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::OnceLock;
    fn data() -> &'static [u64] {
        static DATA: OnceLock<Vec<u64>> = OnceLock::new();
        DATA.get_or_init(|| load_solutions().unwrap())
    }
    // Deliberately plain reference: explicit solution lists, all legal moves,
    // no memoization or size-two shortcut. Sorting only changes enumeration
    // order; every legal child remains eligible for recursive evaluation.
    fn reference(s: &[u64], rule: Rule, p: u8) -> bool {
        reference_placed(s, rule, determined(s).count_ones() as u8 - p)
    }
    fn reference_placed(s: &[u64], rule: Rule, placed: u8) -> bool {
        if s.len() <= 1 {
            return false;
        }
        let d = determined(s);
        let mut children = Vec::new();
        for c in 0..CELLS {
            let child: Vec<_> = s.iter().copied().filter(|m| m & (1u64 << c) != 0).collect();
            if child.is_empty() || child.len() == s.len() {
                continue;
            }
            children.push(child);
        }
        children.sort_by_key(Vec::len);
        for child in children {
            if !reference_placed(&child, rule, placed + 1) {
                return true;
            }
        }
        // Recompute the available passes from the absolute stone count instead
        // of using the fast solver's incremental p transition.
        rule == Rule::Pass
            && d.count_ones() > u32::from(placed)
            && !reference_placed(s, rule, placed + 1)
    }
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
    }
    fn compare_random(rule: Rule) {
        let mut rng = Rng(if rule == Rule::Normal {
            0x182734aabb
        } else {
            0x792145ccdd
        });
        let mut checked = 0;
        let mut seen = HashSet::new();
        let mut solver = Solver::new(rule, 100_000, Arc::new(AtomicU64::new(0)));
        for _ in 0..20_000 {
            let mut s = data().to_vec();
            let mut placed = 0u64;
            let target = 6 + (rng.next() % 5) as usize;
            for depth in 1..=target {
                let d = determined(&s);
                let possible = s.iter().fold(0u64, |a, m| a | m);
                let legal: Vec<_> = (0..CELLS)
                    .filter(|&c| {
                        placed & (1u64 << c) == 0
                            && possible & (1u64 << c) != 0
                            && (d & (1u64 << c) == 0 || rule == Rule::Pass)
                    })
                    .collect();
                if legal.is_empty() {
                    break;
                }
                let c = legal[(rng.next() % legal.len() as u64) as usize];
                placed |= 1u64 << c;
                if d & (1u64 << c) == 0 {
                    s.retain(|m| m & (1u64 << c) != 0);
                }
                if s.len() < 20 {
                    break;
                }
                if depth == target && s.len() <= 400 {
                    let p = (determined(&s).count_ones() - placed.count_ones()) as u8;
                    let key = (determined(&s), if rule == Rule::Pass { p } else { 0 });
                    if !seen.insert(key) {
                        break;
                    }
                    if checked % 25 == 0 {
                        eprintln!(
                            "{} reference comparison: {checked}/200, next n={} p={p}",
                            if rule == Rule::Normal {
                                "normal"
                            } else {
                                "pass"
                            },
                            s.len()
                        );
                    }
                    let expected = reference(&s, rule, p);
                    let (actual, _, _, _) =
                        solver.solve(s.clone(), placed.count_ones() as u8).unwrap();
                    assert_eq!(actual, expected, "stones={placed:#x} n={} p={p}", s.len());
                    checked += 1;
                    if checked == 200 {
                        return;
                    }
                }
            }
        }
        panic!("generated only {checked} qualifying positions");
    }
    #[test]
    fn random_normal_200() {
        compare_random(Rule::Normal);
    }
    #[test]
    fn random_pass_200() {
        compare_random(Rule::Pass);
    }
    #[test]
    fn known_normal() {
        let (mask, placed) = stones("14,18,0,60").unwrap();
        let mut solver = Solver::new(Rule::Normal, 1_000_000, Arc::new(AtomicU64::new(0)));
        let (value, _, n, _) = solver.solve(select(data(), mask), placed).unwrap();
        assert_eq!(n, 3136);
        assert!(
            value,
            "independent checker disagrees with supplied known value"
        );
    }
    #[test]
    fn tiny_rules_and_caps() {
        let sets = [
            vec![3, 5, 9],
            vec![7, 11, 13, 14],
            vec![7, 11, 13], // No singleton move: one available pass flips the value.
            vec![7, 11],
            vec![7],
        ];
        for rule in [Rule::Normal, Rule::Pass] {
            for cap in [0, 1, 100] {
                let mut solver = Solver::new(rule, cap, Arc::new(AtomicU64::new(0)));
                for s in &sets {
                    for placed in 0..=determined(s).count_ones() as u8 {
                        let p = determined(s).count_ones() as u8 - placed;
                        // Different universes must not share a TT; production jobs all
                        // come from the same data universe and can share it safely.
                        solver.tt.clear();
                        assert_eq!(
                            solver.solve(s.clone(), placed).unwrap().0,
                            reference(s, rule, p)
                        );
                    }
                }
            }
        }
        assert!(stones("0,0").is_err());
        assert!(stones("61").is_err());
        assert_eq!(stones("-").unwrap(), (0, 0));
    }
}
