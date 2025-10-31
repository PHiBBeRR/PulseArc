#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms)]
#![warn(clippy::all, clippy::perf, clippy::complexity, clippy::suspicious)]

//! Trie (prefix tree) for string matching and prefix queries.
//!
//! ## Complexity
//! - `insert`, `contains`, `starts_with`, `remove`, and `clear` run in `O(m)`
//!   time where `m` is the number of Unicode scalar values (characters) in the
//!   processed string.
//! - `find_prefix` and `iter_prefix` traverse the branch described by `prefix`
//!   and yield descendants in lexicographic order. The cost is `O(m + t)` for
//!   `t` emitted strings with an additional `O(b log b)` sorting factor per
//!   visited branching node.
//! - `count` and `is_empty` are `O(1)`.
//!
//! ## Panic Safety
//! All operations are panic-free under normal use. Panics can only arise from
//! allocator failures when growing internal `Vec`, `String`, or `HashMap`
//! instances.
//!
//! ## Thread Safety
//! The trie owns no interior mutability. Share it across threads using
//! synchronization primitives (e.g., `Arc<RwLock<Trie>>`) when mutation is
//! required. The type itself is `Send + Sync`.
//!
//! ## Unicode considerations
//! Strings are iterated with `chars()`, so each Unicode scalar value maps to a
//! single edge in the trie. No normalization is applied; callers should
//! normalize inputs when canonical equivalence is required.

use std::collections::HashMap;

/// A trie storing Unicode strings and supporting prefix operations.
///
/// Nodes live inside a `Vec<Node>` and reference children via indices. This
/// keeps cloning cheap and avoids deep pointer chains. Repeated inserts of the
/// same word are idempotent.
///
/// # Examples
///
/// ```
/// use pulsearc_common::collections::Trie;
///
/// let mut trie = Trie::new();
/// trie.insert("alpha");
/// trie.insert("beta");
///
/// assert!(trie.contains("alpha"));
/// assert!(trie.starts_with("alp"));
///
/// let words: Vec<_> = trie.iter_prefix("a").collect();
/// assert_eq!(words, vec!["alpha"]);
/// ```
#[derive(Debug, Clone)]
pub struct Trie {
    nodes: Vec<Node>,
    free_list: Vec<usize>,
    word_count: usize,
}

#[derive(Debug, Clone)]
struct Node {
    children: HashMap<char, usize>,
    terminal: bool,
}

impl Node {
    fn new() -> Self {
        Self { children: HashMap::new(), terminal: false }
    }

    fn reset(&mut self) {
        self.children.clear();
        self.terminal = false;
    }
}

impl Trie {
    /// Creates a new empty trie.
    ///
    /// # Complexity
    /// `O(1)`.
    pub fn new() -> Self {
        Self { nodes: vec![Node::new()], free_list: Vec::new(), word_count: 0 }
    }

    /// Inserts a word into the trie.
    ///
    /// Re-inserting the same word is a no-op.
    ///
    /// # Complexity
    /// `O(m)` where `m` is the number of Unicode scalar values in `word`.
    pub fn insert(&mut self, word: &str) {
        let mut current = 0usize;

        for ch in word.chars() {
            let next = match self.nodes[current].children.get(&ch) {
                Some(&idx) => idx,
                None => {
                    let idx = self.allocate_node();
                    self.nodes[current].children.insert(ch, idx);
                    idx
                }
            };
            current = next;
        }

        if !self.nodes[current].terminal {
            self.nodes[current].terminal = true;
            self.word_count += 1;
        }
    }

    /// Returns `true` if the trie contains the given word.
    ///
    /// # Complexity
    /// `O(m)` where `m` is the number of Unicode scalar values in `word`.
    pub fn contains(&self, word: &str) -> bool {
        self.follow(word).is_some_and(|idx| self.nodes[idx].terminal)
    }

    /// Returns `true` if any stored word starts with `prefix`.
    ///
    /// # Complexity
    /// `O(m)` where `m` is the number of Unicode scalar values in `prefix`.
    pub fn starts_with(&self, prefix: &str) -> bool {
        self.follow(prefix).is_some()
    }

    /// Returns all words that start with `prefix` in lexicographic order.
    ///
    /// # Complexity
    /// `O(m + t)` where `m` is the number of Unicode scalar values in `prefix`
    /// and `t` is the number of emitted strings, plus the `O(b log b)`
    /// ordering cost per branching node.
    pub fn find_prefix(&self, prefix: &str) -> Vec<String> {
        self.iter_prefix(prefix).collect()
    }

    /// Removes a word from the trie, returning `true` if it existed.
    ///
    /// # Complexity
    /// `O(m)` where `m` is the number of Unicode scalar values in `word`.
    pub fn remove(&mut self, word: &str) -> bool {
        let chars: Vec<char> = word.chars().collect();
        let mut current = 0usize;
        let mut path = Vec::with_capacity(chars.len());

        for &ch in &chars {
            let next = match self.nodes[current].children.get(&ch) {
                Some(&idx) => idx,
                None => return false,
            };
            path.push((current, ch, next));
            current = next;
        }

        if !self.nodes[current].terminal {
            return false;
        }

        self.nodes[current].terminal = false;
        self.word_count -= 1;

        while let Some((parent_idx, ch, child_idx)) = path.pop() {
            if self.nodes[child_idx].terminal || !self.nodes[child_idx].children.is_empty() {
                break;
            }

            self.nodes[parent_idx].children.remove(&ch);
            self.recycle_node(child_idx);
        }

        true
    }

    /// Returns the number of distinct words stored in the trie.
    ///
    /// # Complexity
    /// `O(1)`.
    pub fn count(&self) -> usize {
        self.word_count
    }

    /// Returns `true` if the trie contains no words.
    ///
    /// # Complexity
    /// `O(1)`.
    pub fn is_empty(&self) -> bool {
        self.word_count == 0
    }

    /// Removes every word from the trie without deallocating storage.
    ///
    /// # Complexity
    /// `O(n)` over nodes retained by the structure.
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.nodes.push(Node::new());
        self.free_list.clear();
        self.word_count = 0;
    }

    /// Returns an iterator over all words that start with `prefix`.
    ///
    /// The iterator yields values in lexicographic order and only allocates for
    /// the returned `String`s. Traversal reuses a single `String` buffer for
    /// the current path.
    ///
    /// # Complexity
    /// Equivalent to [`find_prefix`](Self::find_prefix) but with lazy
    /// evaluation.
    pub fn iter_prefix(&self, prefix: &str) -> IterPrefix<'_> {
        if let Some((node_idx, is_terminal)) = self.follow_with_terminal(prefix) {
            let entries = self.sorted_children(node_idx);
            IterPrefix {
                trie: self,
                stack: vec![Frame { entries, pos: 0, entered_char: None }],
                current: prefix.to_string(),
                pending_terminal: is_terminal,
                finished: false,
            }
        } else {
            IterPrefix {
                trie: self,
                stack: Vec::new(),
                current: prefix.to_string(),
                pending_terminal: false,
                finished: true,
            }
        }
    }

    fn follow(&self, text: &str) -> Option<usize> {
        let mut current = 0usize;
        for ch in text.chars() {
            current = *self.nodes[current].children.get(&ch)?;
        }
        Some(current)
    }

    fn follow_with_terminal(&self, text: &str) -> Option<(usize, bool)> {
        self.follow(text).map(|idx| (idx, self.nodes[idx].terminal))
    }

    fn sorted_children(&self, index: usize) -> Vec<(char, usize)> {
        let mut entries: Vec<(char, usize)> =
            self.nodes[index].children.iter().map(|(&ch, &idx)| (ch, idx)).collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        entries
    }

    fn allocate_node(&mut self) -> usize {
        if let Some(idx) = self.free_list.pop() {
            self.nodes[idx].reset();
            idx
        } else {
            self.nodes.push(Node::new());
            self.nodes.len() - 1
        }
    }

    fn recycle_node(&mut self, index: usize) {
        self.nodes[index].reset();
        self.free_list.push(index);
    }
}

impl Default for Trie {
    fn default() -> Self {
        Self::new()
    }
}

/// Iterator returned by [`Trie::iter_prefix`].
#[derive(Debug)]
pub struct IterPrefix<'a> {
    trie: &'a Trie,
    stack: Vec<Frame>,
    current: String,
    pending_terminal: bool,
    finished: bool,
}

#[derive(Debug)]
struct Frame {
    entries: Vec<(char, usize)>,
    pos: usize,
    entered_char: Option<char>,
}

impl<'a> Iterator for IterPrefix<'a> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        if self.pending_terminal {
            self.pending_terminal = false;
            return Some(self.current.clone());
        }

        loop {
            if self.stack.is_empty() {
                self.finished = true;
                return None;
            }

            let (ch, child_idx) = {
                let frame = self.stack.last_mut().expect("frame exists");
                if frame.pos >= frame.entries.len() {
                    let frame = self.stack.pop().expect("frame exists");
                    if frame.entered_char.is_some() {
                        self.current.pop();
                    }
                    continue;
                }
                let (ch, child_idx) = frame.entries[frame.pos];
                frame.pos += 1;
                (ch, child_idx)
            };

            self.current.push(ch);

            let entries = self.trie.sorted_children(child_idx);
            let terminal = self.trie.nodes[child_idx].terminal;

            self.stack.push(Frame { entries, pos: 0, entered_char: Some(ch) });

            if terminal {
                return Some(self.current.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for collections::trie.
    use super::Trie;

    // NOTE: Running `cargo test -p pulsearc-common` currently fails due to
    // unrelated compile errors in integration test modules outside this trie
    // implementation.

    /// Validates `Trie::new` behavior for the insert and lookup scenario.
    ///
    /// Assertions:
    /// - Ensures `trie.contains("hello")` evaluates to true.
    /// - Ensures `trie.contains("world")` evaluates to true.
    /// - Ensures `trie.starts_with("hel")` evaluates to true.
    /// - Ensures `!trie.contains("held")` evaluates to true.
    /// - Ensures `trie.starts_with("")` evaluates to true.
    #[test]
    fn insert_and_lookup() {
        let mut trie = Trie::new();

        trie.insert("hello");
        trie.insert("world");
        trie.insert("help");

        assert!(trie.contains("hello"));
        assert!(trie.contains("world"));
        assert!(trie.starts_with("hel"));
        assert!(!trie.contains("held"));
        assert!(trie.starts_with(""));
    }

    /// Validates `Trie::new` behavior for the find prefix sorted scenario.
    ///
    /// Assertions:
    /// - Confirms `words` equals `vec!["alpha", "alphabet", "alphanumeric"]`.
    #[test]
    fn find_prefix_sorted() {
        let mut trie = Trie::new();

        trie.insert("alpha");
        trie.insert("alphabet");
        trie.insert("alphanumeric");
        trie.insert("beta");

        let words = trie.find_prefix("alph");
        assert_eq!(words, vec!["alpha", "alphabet", "alphanumeric"]);
    }

    /// Validates `Trie::new` behavior for the remove leaf and internal
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `trie.remove("cart")` evaluates to true.
    /// - Ensures `!trie.contains("cart")` evaluates to true.
    /// - Ensures `trie.contains("car")` evaluates to true.
    /// - Ensures `trie.contains("cat")` evaluates to true.
    /// - Ensures `trie.remove("car")` evaluates to true.
    /// - Ensures `!trie.contains("car")` evaluates to true.
    /// - Ensures `trie.contains("cat")` evaluates to true.
    /// - Ensures `!trie.remove("car")` evaluates to true.
    #[test]
    fn remove_leaf_and_internal() {
        let mut trie = Trie::new();

        trie.insert("car");
        trie.insert("cart");
        trie.insert("cat");

        assert!(trie.remove("cart"));
        assert!(!trie.contains("cart"));
        assert!(trie.contains("car"));
        assert!(trie.contains("cat"));

        assert!(trie.remove("car"));
        assert!(!trie.contains("car"));
        assert!(trie.contains("cat"));
        assert!(!trie.remove("car"));
    }

    /// Validates `Trie::new` behavior for the unicode strings scenario.
    ///
    /// Assertions:
    /// - Ensures `trie.contains(crab_word)` evaluates to true.
    /// - Ensures `trie.starts_with("na\u{00EF}")` evaluates to true.
    /// - Confirms `words` equals `vec![naive, naive_te]`.
    #[test]
    fn unicode_strings() {
        let mut trie = Trie::new();

        let crab_word = "\u{1F980}rust";
        let naive = "na\u{00EF}ve";
        let naive_te = "na\u{00EF}vet\u{00E9}";

        trie.insert(crab_word);
        trie.insert(naive);
        trie.insert(naive_te);

        assert!(trie.contains(crab_word));
        assert!(trie.starts_with("na\u{00EF}"));

        let mut words = trie.find_prefix("na");
        words.sort();
        assert_eq!(words, vec![naive, naive_te]);
    }

    /// Validates `Trie::new` behavior for the clear and count invariants
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `trie.count()` equals `2`.
    /// - Ensures `!trie.is_empty()` evaluates to true.
    /// - Confirms `trie.count()` equals `0`.
    /// - Ensures `trie.is_empty()` evaluates to true.
    /// - Ensures `!trie.remove("alpha")` evaluates to true.
    #[test]
    fn clear_and_count_invariants() {
        let mut trie = Trie::new();

        trie.insert("alpha");
        trie.insert("alpha");
        trie.insert("beta");

        assert_eq!(trie.count(), 2);
        assert!(!trie.is_empty());

        trie.clear();
        assert_eq!(trie.count(), 0);
        assert!(trie.is_empty());

        assert!(!trie.remove("alpha"));
    }

    /// Validates `Trie::new` behavior for the iter prefix matches find prefix
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `via_iter` equals `via_find`.
    /// - Confirms `via_iter` equals `vec!["app", "apple", "application"]`.
    #[test]
    fn iter_prefix_matches_find_prefix() {
        let mut trie = Trie::new();

        trie.insert("app");
        trie.insert("apple");
        trie.insert("application");
        trie.insert("apt");

        let via_iter: Vec<_> = trie.iter_prefix("app").collect();
        let via_find = trie.find_prefix("app");

        assert_eq!(via_iter, via_find);
        assert_eq!(via_iter, vec!["app", "apple", "application"]);
    }
}
