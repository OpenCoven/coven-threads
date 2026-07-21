//! Emit the public synthetic retired-Ward Phase 5 corpus as canonical JSON.

#[path = "../tests/support/phase5_retired_ward_corpus.rs"]
mod corpus;

fn main() {
    println!("{}", corpus::canonical_corpus_json());
}
