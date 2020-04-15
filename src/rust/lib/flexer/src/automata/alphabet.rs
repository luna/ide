use crate::automata::state::Symbol;

use std::collections::BTreeSet;
use std::ops::RangeInclusive;



// ================
// === Alphabet ===
// ================

/// An alphabet describes a set of all the valid input symbols
/// that a given finite state automata (NFA or DFA) can operate over.
/// More information at: https://en.wikipedia.org/wiki/Deterministic_finite_automaton
/// The alphabet is meant to be represented as an interval.
/// That is, if `a` and `b` are in alphabet,
/// then any symbol from `a..=b` is in alphabet too.
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct Alphabet {
    /// The interval of all valid input symbols.
    /// The interval is further divided into subintervals
    /// (i.e. `[a,z,A,Z]` should be understood as `[a..=z,z..=A,A..=Z]`),
    /// in order to efficiently encode state transitions
    /// that trigger not just on one but a whole range of symbols (i.e. `a..=z`)
    pub symbols: BTreeSet<Symbol>
}

impl Default for Alphabet {
    fn default() -> Self {
        Alphabet {symbols:[Symbol{val:0}].iter().cloned().collect()}
    }
}

impl Alphabet {
    /// Inserts a range of symbols into the alphabet.
    pub fn insert(&mut self, range:RangeInclusive<Symbol>) {
        self.symbols.insert(Symbol{val:range.start().val});
        self.symbols.insert(Symbol{val:range.end().val + 1});
    }
}

impl From<Vec<i64>> for Alphabet {
    fn from(vec:Vec<i64>) -> Self {
        let mut dict = Self::default();
        for val in vec {
            dict.symbols.insert(Symbol{val});
        }
        dict
    }
}
