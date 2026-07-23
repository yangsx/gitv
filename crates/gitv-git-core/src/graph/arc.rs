//! Faithful port of gitk's arc topology system.
//!
//! Arc indices are 1-based, matching gitk's Tcl array convention.
//! Index 0 is the null sentinel.
//!
//! Source reference: /usr/bin/gitk lines 858-1428, 5303-5346

use std::collections::{HashMap, HashSet};

use crate::models::Oid;

/// Convert a non-negative integer to a fixed-sort-order hex string.
/// gitk's `strrep` (line 858).
///
/// Magnitude prefixes: `x` < `y` < `z` ensure lexicographic comparison
/// matches numeric order across different magnitudes.
fn strrep(n: u32) -> String {
    if n < 16 {
        format!("{:x}", n)
    } else if n < 256 {
        format!("x{:02x}", n)
    } else if n < 65536 {
        format!("y{:04x}", n)
    } else {
        format!("z{:08x}", n)
    }
}

/// gitk's arc tree. 1-based indexing, index 0 = null sentinel.
#[derive(Clone, Debug)]
pub struct ArcTree {
    // Tree structure (1-based, 0 = null)
    pub vupptr: Vec<usize>,   // parent arc index
    pub vdownptr: Vec<usize>, // first child arc index
    pub vleftptr: Vec<usize>, // right sibling arc index
    pub vbackptr: Vec<usize>, // left sibling arc index

    // Per-arc data
    pub varcstart: Vec<Oid>,         // [arc_idx] → start commit OID
    pub varctok: Vec<String>,        // [arc_idx] → ordering token
    pub varcrow: Vec<Option<usize>>, // [arc_idx] → row number
    pub varcix: Vec<Option<usize>>,  // [arc_idx] → arc order index
    pub vlastins: Vec<usize>,        // [arc_idx] → last insertion point per up-arc

    // Commit mapping
    pub varcid: HashMap<Oid, usize>,           // OID → arc index
    pub varccommits: HashMap<usize, Vec<Oid>>, // arc_idx → commit list

    // Seed counters (gitk: vseedcount)
    vseedcount: HashMap<u32, i32>, // date → counter

    // Ordertoken cache (gitk: ordertok array)
    pub ordertok: HashMap<Oid, String>,
    pub ordertok_hits: u64,
    pub ordertok_misses: u64,

    // Diagnostics
    pub fix_reversal_calls: u64,
    pub renumber_arc_calls: u64,
    pub split_arc_calls: u64,
    pub sibling_walk_total: u64,
    pub sibling_walk_count: u64,

    // When true, skip fix_reversal during insert_commit (batch mode).
    // The arc tree is built correctly by new_arc's token-based insertion.
    // fix_reversal is only needed for incremental (streaming) addition.
    pub batch_mode: bool, // default true — skip fix_reversal (commits pre-sorted topologically)

    // Layout state
    pub vtokmod: String,
    pub varcmod: usize,
    pub vrowmod: usize,

    // Per-commit data
    pub children: HashMap<Oid, Vec<Oid>>, // OID → children list (display order)
    pub parents: HashMap<Oid, Vec<Oid>>,  // OID → parent list

    pub vrownum: Vec<usize>,   // arc order index → row number
    pub varcorder: Vec<usize>, // arc order index → arc index

    // Display order output
    pub displayorder: Vec<Oid>,
    pub parentlist: Vec<Vec<Oid>>,
}

impl Default for ArcTree {
    fn default() -> Self {
        Self::new()
    }
}

impl ArcTree {
    /// Initialize a new arc tree. gitk's `varcinit` (line 872).
    #[must_use]
    pub fn new() -> Self {
        Self {
            vupptr: vec![0],
            vdownptr: vec![0],
            vleftptr: vec![0],
            vbackptr: vec![0],
            varcstart: vec![Oid::from_bytes([0; 20])],
            varctok: vec![String::new()],
            varcrow: vec![None],
            varcix: vec![None],
            vlastins: vec![0],
            varcid: HashMap::new(),
            varccommits: HashMap::new(),
            vseedcount: HashMap::new(),
            ordertok: HashMap::new(),
            ordertok_hits: 0,
            ordertok_misses: 0,
            fix_reversal_calls: 0,
            renumber_arc_calls: 0,
            split_arc_calls: 0,
            sibling_walk_total: 0,
            sibling_walk_count: 0,
            batch_mode: true,
            vtokmod: String::new(),
            varcmod: 0,
            vrowmod: 0,
            children: HashMap::new(),
            parents: HashMap::new(),
            vrownum: vec![0],
            varcorder: Vec::new(),
            displayorder: Vec::new(),
            parentlist: Vec::new(),
        }
    }

    /// Get the arc token for a commit, or empty string if not found.
    fn token_for(&self, oid: &Oid) -> &str {
        self.varcid
            .get(oid)
            .map(|&i| self.varctok[i].as_str())
            .unwrap_or("")
    }

    /// gitk's `newvarc` (line 928). Create an arc for a commit.
    ///
    /// If the commit has no children, generates a seed token `"s" + strrep(!cdate) + strrep(counter)`.
    /// If the commit has children, inherits the token from the youngest child's arc.
    ///
    /// Returns the new arc index.
    pub fn new_arc(&mut self, id: Oid, commit_time: i64) -> usize {
        let a = self.varctok.len();
        let children = self.children.get(&id).cloned().unwrap_or_default();

        let tok = if children.is_empty() {
            // Seed token: childless commit (branch tip)
            // gitk line 939-949
            let cdate = commit_time.max(0) as u32;
            let counter = self.vseedcount.entry(cdate).or_insert(-1);
            *counter += 1;
            let c = *counter;
            let cdate = cdate ^ 0xffffffff;
            format!("s{}{}", strrep(cdate), strrep(c as u32))
        } else {
            String::new()
        };

        // gitk lines 952-966: inherit from child with highest token.
        // In gitk, children are pre-sorted so last() gives the highest.
        // In batch mode without pre-sorting, we search all children.
        let mut ka = 0;
        let mut best_kid = None;
        for &kid in &children {
            if let Some(&k) = self.varcid.get(&kid)
                && self.varctok[k].as_str() > tok.as_str()
                && (best_kid.is_none()
                    || self.varctok[k] > self.varctok[self.varcid[&best_kid.unwrap()]])
            {
                best_kid = Some(kid);
                ka = k;
            }
        }
        if ka != 0 {
            let kid = best_kid.unwrap();
            let tok2 = self.varctok[ka].clone();
            if let Some(parents) = self.parents.get(&kid) {
                let i = parents.iter().position(|p| *p == id).unwrap_or(0);
                let j = parents.len() - 1 - i;
                let mut inherited = tok2;
                inherited.push_str(&strrep(j as u32));
                return self.insert_arc_in_tree(a, inherited, ka);
            }
        }

        // Original token path (seed or empty)
        self.insert_arc_in_tree(a, tok, ka)
    }

    /// Insert arc `a` with token `tok` into the tree at the correct position
    /// under up-arc `ka`. gitk lines 967-998 (inline in newvarc).
    fn insert_arc_in_tree(&mut self, a: usize, tok: String, ka: usize) -> usize {
        let mut c = self.vlastins[ka];
        let b;
        // gitk: if {$c == 0 || [string compare $tok $varctok($c)] < 0}
        // i.e., tok < varctok[c] → restart from ka (beginning of children)
        if c == 0 || tok.as_str() < self.varctok[c].as_str() {
            c = ka;
            b = self.vdownptr[ka];
        } else {
            b = self.vleftptr[c];
        }
        let mut current_b = b;
        // gitk: while {$b != 0 && [string compare $tok $varctok($b)] >= 0}
        // i.e., advance while tok >= varctok[b]
        let mut walk_steps: u64 = 0;
        while current_b != 0 && tok.as_str() >= self.varctok[current_b].as_str() {
            c = current_b;
            current_b = self.vleftptr[c];
            walk_steps += 1;
        }
        self.sibling_walk_total += walk_steps;
        self.sibling_walk_count += 1;
        let final_b = current_b;

        // Ensure vectors are at least length a+1
        while self.vupptr.len() <= a {
            self.vupptr.push(0);
        }
        while self.vleftptr.len() <= a {
            self.vleftptr.push(0);
        }
        while self.vbackptr.len() <= a {
            self.vbackptr.push(0);
        }
        while self.varcstart.len() <= a {
            self.varcstart.push(Oid::from_bytes([0; 20]));
        }
        while self.varctok.len() <= a {
            self.varctok.push(String::new());
        }
        while self.vdownptr.len() <= a {
            self.vdownptr.push(0);
        }
        while self.varcrow.len() <= a {
            self.varcrow.push(None);
        }
        while self.varcix.len() <= a {
            self.varcix.push(None);
        }
        while self.vlastins.len() <= a {
            self.vlastins.push(0);
        }

        if c == ka {
            self.vdownptr[ka] = a;
            self.vbackptr[a] = 0;
        } else {
            self.vleftptr[c] = a;
            self.vbackptr[a] = c;
        }
        self.vlastins[ka] = a;
        self.vupptr[a] = ka;
        self.vleftptr[a] = final_b;
        if final_b != 0 {
            self.vbackptr[final_b] = a;
        }
        self.varctok[a] = tok;
        self.varcstart[a] = Oid::from_bytes([0; 20]); // will be set by caller
        self.vdownptr[a] = 0;
        self.varcrow[a] = None;
        self.varcix[a] = None;
        self.varccommits.entry(a).or_default();
        self.vlastins[a] = 0;

        a
    }

    /// gitk's `splitvarc` (line 1001). Split arc at commit `p`.
    ///
    /// Called when a commit on an existing arc gets a new child
    /// (fork point), splitting the arc into two.
    pub fn split_arc(&mut self, p: Oid) {
        self.split_arc_calls += 1;
        let Some(&oa) = self.varcid.get(&p) else {
            return;
        };
        let otok = self.varctok[oa].clone();
        let ac = self.varccommits.get(&oa).cloned().unwrap_or_default();
        let i = ac.iter().position(|&id| id == p).unwrap_or(0);
        if i == 0 {
            return; // p is already arc start, nothing to split
        }

        let na = self.varctok.len();
        // "%" sorts before "0" — gitk line 1012
        let tok = format!("{}%{}", otok, strrep(i as u32));

        // Grow vectors
        self.varctok.push(tok);
        self.varcrow.push(None);
        self.varcix.push(None);
        self.varcstart.push(p);
        // Split commits
        let upper: Vec<Oid> = ac[..i].to_vec();
        let lower: Vec<Oid> = ac[i..].to_vec();
        self.varccommits.insert(oa, upper);
        self.varccommits.insert(na, lower);
        for &id in &self.varccommits[&na] {
            self.varcid.insert(id, na);
        }
        self.vdownptr.push(self.vdownptr[oa]);
        self.vlastins.push(self.vlastins[oa]);
        self.vdownptr[oa] = na;
        self.vlastins[oa] = 0;
        self.vupptr.push(oa);
        self.vleftptr.push(0);
        self.vbackptr.push(0);

        // Update parent pointers of all children of the new arc
        let mut b = self.vdownptr[na];
        while b != 0 {
            self.vupptr[b] = na;
            b = self.vleftptr[b];
        }

        if otok <= self.vtokmod {
            self.modify_arc(oa, None);
        }
    }

    /// gitk's `renumbervarc` (line 1037). Renumber arc and related arcs
    /// after a topological change (e.g., split_arc).
    pub fn renumber_arc(&mut self, start_arc: usize) {
        self.renumber_arc_calls += 1;
        if start_arc == 0 {
            return;
        }

        // Walk the entire tree to find related arcs
        let mut todo = Vec::new();
        let mut isrelated: HashMap<usize, bool> = HashMap::new();
        let mut kidchanged: HashMap<usize, bool> = HashMap::new();

        isrelated.insert(start_arc, true);
        kidchanged.insert(start_arc, true);

        let mut a = start_arc;
        while a != 0 {
            if isrelated.contains_key(&a) {
                todo.push(a);
                if let Some(commits) = self.varccommits.get(&a)
                    && let Some(&last_id) = commits.last()
                    && let Some(parents) = self.parents.get(&last_id)
                {
                    for &p in parents {
                        if let Some(&pa) = self.varcid.get(&p) {
                            isrelated.insert(pa, true);
                        }
                    }
                }
            }
            // Pre-order traversal: go down, then left, then up
            let b = self.vdownptr[a];
            if b == 0 {
                let mut current = a;
                while current != 0 {
                    let lb = self.vleftptr[current];
                    if lb != 0 {
                        a = lb;
                        break;
                    }
                    current = self.vupptr[current];
                }
                if current == 0 {
                    a = 0;
                }
            } else {
                a = b;
            }
        }

        let mut sortkids: HashMap<Oid, bool> = HashMap::new();

        for &a in &todo {
            if !kidchanged.contains_key(&a) {
                continue;
            }
            let id = self.varcstart[a];
            // Sort children by arc token
            if let Some(kids) = self.children.get(&id)
                && kids.len() > 1
            {
                let mut sorted = kids.clone();
                sorted.sort_by(|a, b| self.token_for(a).cmp(self.token_for(b)));
                self.children.insert(id, sorted);
            }
            let oldtok = self.varctok[a].clone();

            // Inherit from last child (youngest, rightmost)
            let mut tok = String::new();
            let mut ka = 0;
            if let Some(kids) = self.children.get(&id)
                && let Some(&kid) = kids.last()
                && let Some(&k) = self.varcid.get(&kid)
                && self.varctok[k] > tok
            {
                ka = k;
                tok = self.varctok[k].clone();
            }
            if ka != 0
                && let Some(&ki) = self.children.get(&id).and_then(|k| k.last())
                && let Some(parents) = self.parents.get(&ki)
            {
                let i = parents.iter().position(|&p| p == id).unwrap_or(0);
                let j = parents.len() - 1 - i;
                tok.push_str(&strrep(j as u32));
            }

            if tok == oldtok {
                continue;
            }

            // Mark parents as needing re-sort
            if let Some(commits) = self.varccommits.get(&a)
                && let Some(&last_id) = commits.last()
                && let Some(parents) = self.parents.get(&last_id)
            {
                for &p in parents {
                    if self.varcid.contains_key(&p) {
                        let pa = self.varcid[&p];
                        kidchanged.insert(pa, true);
                    } else {
                        sortkids.insert(p, true);
                    }
                }
            }

            // Move arc to new position in tree if token changed
            self.varctok[a] = tok.clone();
            let b = self.vupptr[a];
            if b != ka {
                if self.varctok[ka] < self.vtokmod {
                    self.modify_arc(ka, None);
                }
                if self.varctok[b] < self.vtokmod {
                    self.modify_arc(b, None);
                }
                // Remove from old position
                let c = self.vbackptr[a];
                let d = self.vleftptr[a];
                if c == 0 {
                    self.vdownptr[b] = d;
                } else {
                    self.vleftptr[c] = d;
                }
                if d != 0 {
                    self.vbackptr[d] = c;
                }
                if self.vlastins[b] == a {
                    self.vlastins[b] = c;
                }
                // Insert at new position
                self.vupptr[a] = ka;
                let mut c2 = self.vlastins[ka];
                let b2;
                if c2 == 0 || tok.as_str() < self.varctok[c2].as_str() {
                    c2 = ka;
                    b2 = self.vdownptr[ka];
                } else {
                    b2 = self.vleftptr[c2];
                }
                let mut current_b = b2;
                while current_b != 0 && tok.as_str() >= self.varctok[current_b].as_str() {
                    c2 = current_b;
                    current_b = self.vleftptr[c2];
                }
                if c2 == ka {
                    self.vdownptr[ka] = a;
                    self.vbackptr[a] = 0;
                } else {
                    self.vleftptr[c2] = a;
                    self.vbackptr[a] = c2;
                }
                self.vleftptr[a] = current_b;
                if current_b != 0 {
                    self.vbackptr[current_b] = a;
                }
                self.vlastins[ka] = a;
            }
        }

        // Sort children of affected parents
        for id in sortkids.keys() {
            if let Some(kids) = self.children.get(id)
                && kids.len() > 1
            {
                let mut sorted = kids.clone();
                sorted.sort_by(|a, b| self.token_for(a).cmp(self.token_for(b)));
                self.children.insert(*id, sorted);
            }
        }
    }

    /// gitk's `fix_reversal` (line 1169). Handle the case where a parent
    /// commit `p` (already in the arc tree) now has a new child in arc `a`
    /// that causes a token ordering reversal.
    pub fn fix_reversal(&mut self, p: Oid, a: usize) {
        self.fix_reversal_calls += 1;
        let Some(&pa) = self.varcid.get(&p) else {
            return;
        };
        if p != self.varcstart[pa] {
            self.split_arc(p);
        }
        let pa2 = self.varcid[&p];
        // Seeds (root arcs with vupptr=0) always need renumbering
        if self.vupptr[pa2] == 0 || self.varctok[a] > self.varctok[pa2] {
            self.renumber_arc(pa2);
        }
    }

    /// gitk's `modify_arc` (line 1333). Mark an arc as modified,
    /// invalidating downstream layout.
    ///
    /// When `lim` is `Some`, acts as a guard: returns early if the arc's
    /// token is already after `vtokmod`, or if equal and the existing row
    /// coverage (`varcrow[a] + lim`) already reaches `vrowmod`.
    ///
    /// When `lim` is `None`, the caller must have verified that the arc's
    /// token is less than `vtokmod` (matching gitk's contract).
    pub fn modify_arc(&mut self, a: usize, lim: Option<usize>) {
        // gitk lines 1338-1346: guard when lim is provided
        if let Some(lim) = lim {
            match self.varctok[a].cmp(&self.vtokmod) {
                std::cmp::Ordering::Greater => return,
                std::cmp::Ordering::Equal => {
                    if let Some(r) = self.varcrow[a]
                        && self.vrowmod <= r + lim
                    {
                        return;
                    }
                }
                std::cmp::Ordering::Less => {}
            }
        }

        self.vtokmod = self.varctok[a].clone();
        self.varcmod = a;

        // Walk up to find the earliest ancestor with a row number.
        // gitk clears `lim` during the walk so that the commit count of the
        // ancestor is used instead of the original caller-supplied value.
        let mut current = a;
        let mut lim = lim;
        while current != 0 && self.varcrow[current].is_none() {
            current = self.vupptr[current];
            lim = None;
        }

        let r = if current == 0 {
            0
        } else {
            let row = self.varcrow[current].unwrap_or(0);
            let effective_lim =
                lim.unwrap_or_else(|| self.varccommits.get(&current).map_or(0, |c| c.len()));
            row + effective_lim
        };

        self.vrowmod = r;
    }

    /// gitk's `update_arcrows` (line 1363). Pre-order tree traversal
    /// to assign row numbers to arcs.
    ///
    /// Returns the total number of rows.
    pub fn update_arcrows(&mut self) -> usize {
        // gitk line 1378: if {$vrowmod($v) == $commitidx($v)} return
        if self.vrowmod == self.varcid.len() {
            return self.vrowmod;
        }

        // Clear displayorder and parentlist for rebuild
        self.displayorder.clear();
        self.parentlist.clear();

        // Find starting arc
        let mut a = self.varcmod;
        while a != 0 && self.varcix[a].is_none() {
            a = self.vupptr[a];
        }

        if a == 0 {
            a = self.vdownptr[0];
            if a == 0 {
                return 0;
            }
            self.vrownum = vec![0];
            self.varcorder = vec![a];
            self.varcix[a] = Some(0);
            self.varcrow[a] = Some(0);
            // arcn = 0, row = 0
        } else {
            let arcn = self.varcix[a].unwrap_or(0);
            // Truncate vrownum and varcorder to arcn
            if self.vrownum.len() > arcn + 1 {
                self.vrownum.truncate(arcn + 1);
                self.varcorder.truncate(arcn + 1);
            }
            // row = varcrow[a]
        }

        let mut arcn = self.varcorder.len() - 1;
        let mut row = self.varcrow[a].unwrap_or(0);
        let mut last_p = a;
        let mut visited: HashSet<usize> = HashSet::new();
        let max_iter = self.varctok.len().saturating_mul(2).max(1000);

        loop {
            if !visited.insert(a) {
                break;
            }
            if visited.len() > max_iter {
                break;
            }
            last_p = a;
            let commit_count = self.varccommits.get(&a).map_or(0, |c| c.len());
            row += commit_count;

            // Go down if possible
            let mut b = self.vdownptr[a];
            if b == 0 {
                // If not, go left, or go up until we can go left
                let mut current = a;
                while current != 0 {
                    b = self.vleftptr[current];
                    if b != 0 {
                        break;
                    }
                    current = self.vupptr[current];
                }
                if current == 0 {
                    break;
                }
            }
            a = b;
            arcn += 1;
            // Grow vrownum and varcorder if needed
            while self.vrownum.len() <= arcn {
                self.vrownum.push(0);
            }
            while self.varcorder.len() <= arcn {
                self.varcorder.push(0);
            }
            self.vrownum[arcn] = row;
            self.varcorder[arcn] = a;
            // Grow varcix and varcrow if needed
            while self.varcix.len() <= a {
                self.varcix.push(None);
            }
            while self.varcrow.len() <= a {
                self.varcrow.push(None);
            }
            self.varcix[a] = Some(arcn);
            self.varcrow[a] = Some(row);
        }

        self.vtokmod = self.varctok[last_p].clone();
        self.varcmod = last_p;
        self.vrowmod = row;

        row
    }

    /// gitk's `make_disporder` (line 1501). Build flat displayorder
    /// and parentlist for rows `[start, end)`.
    ///
    /// Returns `(displayorder, parentlist)`.
    pub fn make_disporder(&mut self, start: usize, end: usize) -> (Vec<Oid>, Vec<Vec<Oid>>) {
        if end > self.vrowmod {
            self.update_arcrows();
        }

        // Binary search for the starting arc
        let ai = self
            .vrownum
            .binary_search(&start)
            .unwrap_or_else(|i| i.saturating_sub(1));
        let start = self.vrownum[ai]; // adjusted start
        let narc = self.vrownum.len();

        self.displayorder.clear();
        self.parentlist.clear();
        let mut r = start;

        for ai in ai..narc {
            if r >= end {
                break;
            }
            let a = self.varcorder[ai];
            let al = self.varccommits.get(&a).map_or(0, |c| c.len());
            let l = self.displayorder.len();

            if l < r + al {
                if l < r {
                    // Pad with empty entries
                    self.displayorder.resize(r, Oid::from_bytes([0; 20]));
                    self.parentlist.resize(r, Vec::new());
                } else if l > r {
                    // Truncate
                    self.displayorder.truncate(r);
                    self.parentlist.truncate(r);
                }
                for &id in self.varccommits.get(&a).unwrap_or(&vec![]) {
                    self.displayorder.push(id);
                    self.parentlist
                        .push(self.parents.get(&id).cloned().unwrap_or_default());
                }
            } else if self
                .displayorder
                .get(r + al - 1)
                .is_none_or(|&id| id == Oid::from_bytes([0; 20]))
            {
                for (i, &id) in (r..).zip(self.varccommits.get(&a).unwrap_or(&vec![]).iter()) {
                    if i < self.displayorder.len() {
                        self.displayorder[i] = id;
                        self.parentlist[i] = self.parents.get(&id).cloned().unwrap_or_default();
                    } else {
                        self.displayorder.push(id);
                        self.parentlist
                            .push(self.parents.get(&id).cloned().unwrap_or_default());
                    }
                }
            }
            r += al;
        }

        (self.displayorder.clone(), self.parentlist.clone())
    }

    /// gitk's `first_real_child` (line 1300).
    /// First non-null child of a commit.
    fn first_real_child(&self, id: Oid) -> Option<Oid> {
        self.children
            .get(&id)
            .and_then(|kids| kids.first().copied())
    }

    /// gitk's `last_real_child` (line 1311).
    /// Last non-null child of a commit.
    #[allow(dead_code)]
    fn last_real_child(&self, id: Oid) -> Option<Oid> {
        self.children.get(&id).and_then(|kids| kids.last().copied())
    }

    /// gitk's `ordertoken` (line 5303). Walk up arcs to compute
    /// a sort key for a commit. Results are cached in `self.ordertok`.
    ///
    /// Walks UP through the DAG to find the base token (from an arc's varctok),
    /// then walks back DOWN appending parent-index segments.
    /// Intermediate tokens are cached so subsequent calls are O(1).
    pub fn ordertoken(&mut self, id: Oid) -> String {
        // gitk line 5307: check cache first
        if let Some(cached) = self.ordertok.get(&id) {
            self.ordertok_hits += 1;
            return cached.clone();
        }
        self.ordertok_misses += 1;

        let origid = id;
        let mut todo: Vec<(Oid, String)> = Vec::new();
        let mut current_id = id;
        let tok;

        // Walk up (gitk lines 5312-5338)
        loop {
            // gitk lines 5312-5317: determine p
            let p = if let Some(&a) = self.varcid.get(&current_id) {
                self.varcstart[a]
            } else {
                // current_id not in arc — use first child (gitk: children[id][0])
                match self.first_real_child(current_id) {
                    Some(child) => child,
                    None => {
                        // No children and not in arc: use arc token if available.
                        // (This case shouldn't occur in normal operation.)
                        tok = self
                            .varcid
                            .get(&current_id)
                            .map(|&a| self.varctok[a].clone())
                            .unwrap_or_default();
                        break;
                    }
                }
            };

            // gitk line 5319: check cache for p
            if let Some(cached) = self.ordertok.get(&p) {
                tok = cached.clone();
                break;
            }

            // gitk lines 5323-5337: walk to p's first_real_child
            match self.first_real_child(p) {
                None => {
                    // p has no children — it's a root/seed
                    // gitk: set tok [lindex $varctok($v) $varcid($v,$p)]
                    tok = self
                        .varcid
                        .get(&p)
                        .map(|&a| self.varctok[a].clone())
                        .unwrap_or_default();
                    break;
                }
                Some(child) => {
                    // gitk lines 5329-5337: record segment
                    let segment = match self.parents.get(&child) {
                        Some(parents) if parents.len() == 1 => String::new(),
                        Some(parents) => {
                            let j = parents.iter().position(|&par| par == p).unwrap_or(0);
                            strrep(j as u32)
                        }
                        None => String::new(),
                    };
                    todo.push((p, segment));
                    current_id = child;
                }
            }
        }

        // gitk lines 5340-5344: walk back down, append segments, cache
        let mut result = tok;
        for (p_oid, segment) in todo.iter().rev() {
            result.push_str(segment);
            self.ordertok.insert(*p_oid, result.clone());
        }
        // gitk line 5345: cache the original commit
        self.ordertok.insert(origid, result.clone());
        result
    }

    /// gitk's `vtokcmp` (line 1324). Compare two commits by arc token.
    #[allow(dead_code)]
    fn vtokcmp(&self, a: Oid, b: Oid) -> std::cmp::Ordering {
        self.token_for(&a).cmp(self.token_for(&b))
    }

    /// Main entry: insert a commit into the arc tree.
    /// gitk's inline logic in `getcommitlines` (lines 1779-1829).
    pub fn insert_commit(&mut self, id: Oid, parent_oids: &[Oid], commit_time: i64) {
        let already_inserted = self.varcid.contains_key(&id);

        // Store parents
        self.parents.insert(id, parent_oids.to_vec());

        // Determine arc: try to join child's arc
        let mut a = 0;
        if let Some(kids) = self.children.get(&id)
            && !already_inserted
            && kids.len() == 1
        {
            let k = kids[0];
            if let Some(&ka) = self.varcid.get(&k)
                && let Some(parents) = self.parents.get(&k)
                && parents.len() == 1
            {
                a = ka;
            }
        }

        if a == 0 {
            // New arc
            a = self.new_arc(id, commit_time);
            // Set the arc start commit
            self.varcstart[a] = id;
        }

        if self.varctok[a] < self.vtokmod {
            self.modify_arc(a, None);
        }

        if !already_inserted {
            self.varcid.insert(id, a);
            self.varccommits.entry(a).or_default().push(id);
        }

        // Add to parents' children lists (gitk lines 1813-1829)
        // In batch mode, skip sorting — children are sorted once in a final
        // pass after all insertions (sort_children_all). This avoids O(K²)
        // re-sorting for parents with many children.
        for (i, &p) in parent_oids.iter().enumerate() {
            if i == 0 || !parent_oids[..i].contains(&p) {
                // push without contains() check — the dedup guard above
                // prevents duplicate parents within the same commit
                self.children.entry(p).or_default().push(id);
                if !self.batch_mode {
                    let needs_sort = {
                        let kids = &self.children[&p];
                        kids.len() > 1 && {
                            let prev = kids[kids.len() - 2];
                            self.token_for(&prev) > self.token_for(&id)
                        }
                    };
                    if needs_sort {
                        let mut sorted = self.children[&p].clone();
                        sorted.sort_by(|a, b| self.token_for(a).cmp(self.token_for(b)));
                        self.children.insert(p, sorted);
                    }
                }
                if !self.batch_mode && self.varcid.contains_key(&p) {
                    self.fix_reversal(p, a);
                }
            }
        }
    }

    /// Sort all children lists by arc token. Called once after all
    /// `insert_commit` calls in batch mode.
    pub fn sort_children_all(&mut self) {
        // Collect token for each OID first to avoid borrow conflicts
        let oid_tokens: Vec<(Oid, String)> = self
            .varcid
            .iter()
            .map(|(&oid, &arc)| (oid, self.varctok[arc].clone()))
            .collect();
        let token_map: HashMap<Oid, String> = oid_tokens.into_iter().collect();
        let empty = String::new();
        for kids in self.children.values_mut() {
            kids.sort_by(|a, b| {
                token_map
                    .get(a)
                    .unwrap_or(&empty)
                    .cmp(token_map.get(b).unwrap_or(&empty))
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strrep() {
        assert_eq!(strrep(0), "0");
        assert_eq!(strrep(15), "f");
        assert_eq!(strrep(16), "x10");
        assert_eq!(strrep(255), "xff");
        assert_eq!(strrep(256), "y0100");
        assert_eq!(strrep(65535), "yffff");
        assert_eq!(strrep(65536), "z00010000");
    }

    #[test]
    fn test_vector_alignment_after_insert() {
        let mut tree = ArcTree::new();
        // Insert 3 childless commits (separate seed arcs)
        tree.insert_commit(make_oid(1), &[], 1000);
        tree.insert_commit(make_oid(2), &[], 2000);
        tree.insert_commit(make_oid(3), &[], 3000);

        // All per-arc vectors must be the same length
        let expected_len = tree.varctok.len();
        assert_eq!(
            tree.vupptr.len(),
            expected_len,
            "vupptr misaligned: {} vs varctok {}",
            tree.vupptr.len(),
            expected_len
        );
        assert_eq!(
            tree.vdownptr.len(),
            expected_len,
            "vdownptr misaligned: {} vs varctok {}",
            tree.vdownptr.len(),
            expected_len
        );
        assert_eq!(
            tree.vleftptr.len(),
            expected_len,
            "vleftptr misaligned: {} vs varctok {}",
            tree.vleftptr.len(),
            expected_len
        );
        assert_eq!(
            tree.vbackptr.len(),
            expected_len,
            "vbackptr misaligned: {} vs varctok {} — vbackptr grew by an extra element per insert_arc_in_tree call",
            tree.vbackptr.len(),
            expected_len
        );
        assert_eq!(
            tree.varcstart.len(),
            expected_len,
            "varcstart misaligned: {} vs varctok {}",
            tree.varcstart.len(),
            expected_len
        );
        assert_eq!(
            tree.vlastins.len(),
            expected_len,
            "vlastins misaligned: {} vs varctok {}",
            tree.vlastins.len(),
            expected_len
        );
    }

    fn make_oid(n: u8) -> Oid {
        let mut bytes = [0u8; 20];
        bytes[0] = n;
        Oid::from_bytes(bytes)
    }

    #[test]
    fn test_new_arc_seed_token() {
        let mut tree = ArcTree::new();
        let oid = make_oid(1);
        let a = tree.new_arc(oid, 1000);
        assert!(tree.varctok[a].starts_with('s'));
        assert_eq!(tree.varcstart[a], Oid::from_bytes([0; 20])); // Not set by new_arc itself
    }

    #[test]
    fn test_simple_linear_chain() {
        let mut tree = ArcTree::new();
        // Commits: root <- child1 <- child2 (child2 is newest, row 0)
        let root = make_oid(3);
        let child1 = make_oid(2);
        let child2 = make_oid(1);

        // Insert in display order (newest first): child2, child1, root
        tree.insert_commit(child2, &[child1], 3000);
        tree.insert_commit(child1, &[root], 2000);
        tree.insert_commit(root, &[], 1000);

        let rows = tree.update_arcrows();
        let (displayorder, parentlist) = tree.make_disporder(0, rows);

        // Should be: child2, child1, root (newest to oldest)
        assert_eq!(displayorder.len(), 3);
        assert_eq!(displayorder[0], child2);
        assert_eq!(displayorder[1], child1);
        assert_eq!(displayorder[2], root);
        assert_eq!(parentlist[0], vec![child1]);
        assert_eq!(parentlist[1], vec![root]);
        assert_eq!(parentlist[2], Vec::<Oid>::new());
    }

    #[test]
    fn test_fork_two_branches() {
        let mut tree = ArcTree::new();
        // root <- main <- main2  (mainline)
        // root <- branch <- branch2  (side branch)
        let root = make_oid(5);
        let main = make_oid(4);
        let branch = make_oid(3);
        let main2 = make_oid(2);
        let branch2 = make_oid(1);

        // Insert newest first
        tree.insert_commit(branch2, &[branch], 5000);
        tree.insert_commit(main2, &[main], 4000);
        tree.insert_commit(branch, &[root], 3000);
        tree.insert_commit(main, &[root], 2000);
        tree.insert_commit(root, &[], 1000);

        let rows = tree.update_arcrows();
        let (displayorder, _) = tree.make_disporder(0, rows);

        assert_eq!(displayorder.len(), 5);
        // Newest commits should be at the top
        assert_eq!(displayorder[0], branch2);
        assert_eq!(displayorder[1], branch);
        assert_eq!(displayorder[2], main2);
        assert_eq!(displayorder[3], main);
        assert_eq!(displayorder[4], root);

        // Verify ordertokens: main2 and branch2 are both childless seed tokens
        let tok_main2 = tree.ordertoken(main2);
        let tok_branch2 = tree.ordertoken(branch2);
        // Both should be non-empty (seed tokens)
        assert!(!tok_main2.is_empty());
        assert!(!tok_branch2.is_empty());
        // They should be different (different commit times)
        assert_ne!(tok_main2, tok_branch2);
        // Lexicographic order: newer commit (branch2, time 5000) sorts first
        assert!(
            tok_branch2 < tok_main2,
            "branch2 (newer) should sort before main2 (older)"
        );
    }

    #[test]
    fn test_merge_two_parents() {
        let mut tree = ArcTree::new();
        // root <- branch <- merge (merge has two parents: branch and main)
        // root <- main
        let root = make_oid(4);
        let main = make_oid(3);
        let branch = make_oid(2);
        let merge = make_oid(1);

        tree.insert_commit(merge, &[branch, main], 4000);
        tree.insert_commit(branch, &[root], 3000);
        tree.insert_commit(main, &[root], 2000);
        tree.insert_commit(root, &[], 1000);

        let rows = tree.update_arcrows();
        let (displayorder, parentlist) = tree.make_disporder(0, rows);

        assert_eq!(displayorder.len(), 4);
        assert_eq!(parentlist[0], vec![branch, main]);
    }

    #[test]
    fn test_ordertoken_mainline() {
        let mut tree = ArcTree::new();
        let root = make_oid(3);
        let child = make_oid(2);
        let grandchild = make_oid(1);

        tree.insert_commit(grandchild, &[child], 3000);
        tree.insert_commit(child, &[root], 2000);
        tree.insert_commit(root, &[], 1000);

        let tok = tree.ordertoken(grandchild);
        // Mainline commits should have empty or short tokens
        assert!(tok.is_empty() || tok.starts_with('s'));
    }
}
