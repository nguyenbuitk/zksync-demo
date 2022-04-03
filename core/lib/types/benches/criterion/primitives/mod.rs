// External uses
use criterion::{black_box, criterion_group, BatchSize, Bencher, Criterion, Throughput};
// Local uses
use zksync_crypto::circuit::account::CircuitAccount;
use zksync_crypto::primitives::{BitConvert, BitIteratorLe, GetBits};
use zksync_types::{Account, Address, Nonce, PubKeyHash, TokenId};

/// Input size for byte slices (module-wide for calculating the throughput).
const BYTE_SLICE_SIZE: usize = 512;

fn bench_u64_get_bits_le(b: &mut Bencher<'_>) {
    let value: u64 = 0xDEAD_BEEF_DEAD_BEEF;

    b.iter(|| {
        let _ = black_box(value).get_bits_le();
    });
}

fn bench_bytes_into_be_bits(b: &mut Bencher<'_>) {
    let value: Vec<u8> = vec![0xAB; BYTE_SLICE_SIZE];

    let value_ref: &[u8] = value.as_ref();

    b.iter(|| {
        let _ = BitConvert::from_be_bytes(black_box(value_ref));
    });
}

fn bench_pack_bits_into_bytes(b: &mut Bencher<'_>) {
    let value: Vec<bool> = vec![true; BYTE_SLICE_SIZE * 8];

    let setup = || value.clone();

    b.iter_batched(
        setup,
        |value| {
            let _ = BitConvert::into_bytes(black_box(value));
        },
        BatchSize::SmallInput,
    );
}

fn bench_pack_bits_into_bytes_in_order(b: &mut Bencher<'_>) {
    let value: Vec<bool> = vec![true; BYTE_SLICE_SIZE * 8];

    let setup = || value.clone();

    b.iter_batched(
        setup,
        |value| {
            let _ = BitConvert::into_bytes_ordered(black_box(value));
        },
        BatchSize::SmallInput,
    );
}

fn bench_bit_iterator_le_next(b: &mut Bencher<'_>) {
    let value: Vec<u64> = vec![0xDEAD_BEEF_DEAD_BEEF; BYTE_SLICE_SIZE / 8];

    let setup = || BitIteratorLe::new(&value);

    b.iter_batched(
        setup,
        |bit_iterator| {
            for _ in bit_iterator {
                // Do nothing, we're just draining the iterator.
            }
        },
        BatchSize::SmallInput,
    );
}

fn bench_circuit_account_transform(b: &mut Bencher<'_>) {
    let setup = || {
        let mut account = Account::default_with_address(&Address::from_slice(
            &hex::decode("0102030405060708091011121314151617181920").unwrap(),
        ));
        account.set_balance(TokenId(1), 1u32.into());
        account.set_balance(TokenId(2), 2u32.into());
        account.nonce = Nonce(3);
        account.pub_key_hash =
            PubKeyHash::from_hex("sync:0102030405060708091011121314151617181920").unwrap();
        account
    };

    b.iter_batched(
        setup,
        |account| {
            let _ = CircuitAccount::from(black_box(account));
        },
        BatchSize::SmallInput,
    );
}

/// Measures time to execute `CircuitAccount::default()`.
fn circuit_account_default(b: &mut Bencher<'_>) {
    b.iter_with_large_drop(|| {
        let _ = black_box(CircuitAccount::default());
    });
}

/// Measures time to execute `CircuitAccount::default().get_bits_le()`.
fn bench_circuit_account_get_bits_le_default(b: &mut Bencher<'_>) {
    let circuit_account = CircuitAccount::default();
    let setup = || circuit_account.clone();

    b.iter_batched_ref(
        setup,
        |circuit_account| {
            let _ = black_box(circuit_account.get_bits_le());
        },
        BatchSize::SmallInput,
    );
}

/// `get_bits_le` is a method internally used by SMT to calculate the hash of the element.
///
/// `n_balances` parameter specifies the amount of elements in the balance tree.
fn bench_circuit_account_get_bits_le(b: &mut Bencher<'_>, n_balances: usize) {
    let mut account = Account::default_with_address(&Address::from_slice(
        &hex::decode("0102030405060708091011121314151617181920").unwrap(),
    ));

    for i in (0..n_balances).map(|i| i as u32) {
        account.set_balance(TokenId(i), i.into());
    }
    account.nonce = Nonce(3);
    account.pub_key_hash =
        PubKeyHash::from_hex("sync:0102030405060708091011121314151617181920").unwrap();

    let circuit_account = CircuitAccount::from(account);

    let setup = || circuit_account.clone();

    b.iter_batched_ref(
        setup,
        |circuit_account| {
            let _ = black_box(circuit_account.get_bits_le());
        },
        BatchSize::SmallInput,
    );
}

pub fn bench_primitives(c: &mut Criterion) {
    c.bench_function("u64_get_bits_le", bench_u64_get_bits_le);

    let mut group = c.benchmark_group("Bit Converters");

    group.throughput(Throughput::Bytes(BYTE_SLICE_SIZE as u64));
    group.bench_function("bytes_into_be_bits", bench_bytes_into_be_bits);
    group.bench_function("pack_bits_into_bytes", bench_pack_bits_into_bytes);
    group.bench_function(
        "pack_bits_into_bytes_in_order",
        bench_pack_bits_into_bytes_in_order,
    );
    group.bench_function("BitIterator::next", bench_bit_iterator_le_next);

    group.finish();

    c.bench_function(
        "bench_circuit_account_transform",
        bench_circuit_account_transform,
    );

    c.bench_function("circuit_account_default", circuit_account_default);

    let mut group = c.benchmark_group("bench_circuit_account_get_bits_le");
    group.bench_function("default account", bench_circuit_account_get_bits_le_default);
    for n_balances in [0, 10, 100, 1000] {
        group.bench_function(&format!("n_balances: {}", n_balances), |b| {
            bench_circuit_account_get_bits_le(b, n_balances)
        });
    }
    group.finish();
}

criterion_group!(primitives_benches, bench_primitives);
