use criterion::criterion_main;

use merkle_tree::merkle_tree_benches;
use primitives::primitives_benches;
use signatures::signature_benches;
use txs::txs_benches;

mod merkle_tree;
mod primitives;
mod signatures;
mod txs;

criterion_main!(
    merkle_tree_benches,
    primitives_benches,
    signature_benches,
    txs_benches
);
