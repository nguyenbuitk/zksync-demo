use std::collections::HashMap;
// External uses
// Workspace uses
use zksync_types::{Account, AccountId, AccountTree, AccountUpdates, Address, BlockNumber};
// Local uses
use super::db::StateRestoreDb;

/// `RestoredTree` is an entity capable of restoring the account tree to the latest observed state
/// using the database.
///
/// By default, it will try to load the last tree cache and update from there by loading the state difference.
/// If there is no cache, tree will be recalculated from scratch.
///
/// If the tree root hash will not match the hash from the database, `RestoredTree` will find the block
/// at which hashes diverged and panic with the corresponding message containing the block number.
#[derive(Debug)]
pub(crate) struct RestoredTree<S: StateRestoreDb> {
    pub(crate) storage: S,

    pub(crate) tree: AccountTree,
    pub(crate) acc_id_by_addr: HashMap<Address, AccountId>,
}

impl<S> RestoredTree<S>
where
    S: StateRestoreDb,
{
    pub(crate) fn new(storage: S) -> Self {
        Self {
            storage,

            tree: AccountTree::new(zksync_crypto::params::account_tree_depth()),
            acc_id_by_addr: HashMap::default(),
        }
    }

    /// Restores the tree state.
    /// Returns the block number to which the state was initialized.
    /// This block number is guaranteed to be the last committed block.
    pub(crate) async fn restore(&mut self) -> BlockNumber {
        let last_block = self.storage.load_last_committed_block().await;

        if let Some(cached_block) = self.storage.load_last_cached_block().await {
            self.init_tree_with_cache(cached_block).await;
            self.assert_calculated_root(
                "Root hash from the cached tree doesn't match the root hash from the database",
                cached_block,
            )
            .await;

            // We may not be at the latest point in time.
            // If so, we need to load the state diff and apply it to the tree.
            if let Some(diff) = self.storage.load_state_diff(cached_block, last_block).await {
                self.apply_state_diff(last_block, diff).await;
            }
        } else {
            self.init_tree_without_cache(last_block).await;
        };

        // Now we *must* have the newest tree state. At this point we should check the root hash
        // and ensure that it corresponds to the previously calculated root hash that is already stored in
        // the database.
        let root_hash_from_tree = self.tree.root_hash();
        let root_hash_from_db = self.storage.load_block_hash_from_db(last_block).await;
        if root_hash_from_tree != root_hash_from_db {
            // Root hash from the database doesn't match the hash we calculated now.
            // This is an extreme situation meaning that there is some horrible error
            // in the application logic, so to help developers identify the cause, we get back
            // to the point at which we started and apply the blocks diff one-by-one until we
            // precisely find the block at which root hash doesn't match.
            self.find_hash_mismatch_point().await;
        }

        // At this point tree is restored and is checked to be correct.
        // Store the cache to speed up the future restarts.
        self.storage
            .store_account_tree_cache(last_block, self.tree.get_internals())
            .await;

        last_block
    }

    async fn init_tree_with_cache(&mut self, cache_block: BlockNumber) {
        let committed_state = self.storage.load_committed_state(cache_block).await;
        let cache = self.storage.load_account_tree_cache(cache_block).await;

        for (id, account) in committed_state {
            self.insert_account(id, account);
        }
        self.tree.set_internals(cache);
    }

    async fn init_tree_without_cache(&mut self, last_block_number: BlockNumber) {
        // If we don't have cache we have no other choice rather than load the latest state and recalculate the tree
        // from scratch.
        let committed_state = self.storage.load_committed_state(last_block_number).await;

        for (id, account) in committed_state {
            self.insert_account(id, account);
        }
    }

    /// This function should be called when the resulting hash at the latest state doesn't match the root hash
    /// for the last block in the database.
    ///
    /// It loads the verified state (because the root hash for the last verified state was checked by circuit
    /// to be correct) and applies blocks from it one by one in order to find the block at which hashes
    /// diverged.
    ///
    /// This function is very slow, but it's OK since the server can not start with an incorrect state anyway.
    async fn find_hash_mismatch_point(&mut self) -> ! {
        // Reset self state, we're starting from scratch.
        self.tree = AccountTree::new(zksync_crypto::params::account_tree_depth());
        self.acc_id_by_addr = HashMap::new();

        let (current_block, verified_state) = self.storage.load_verified_state().await;
        let last_block = self.storage.load_last_committed_block().await;

        // Initialize at the verified state.
        for (id, account) in verified_state {
            self.insert_account(id, account);
        }

        // Go through each block, apply state diff, and check the root hash.
        for block in (current_block.0..last_block.0).map(BlockNumber) {
            let next_block = block + 1;
            let diff = self.storage.load_state_diff(block, next_block).await;
            if let Some(diff) = diff {
                self.apply_state_diff(next_block, diff).await;
            }
            self.assert_calculated_root("Root hashes diverged", next_block)
                .await;
        }

        // If this function has been called, hashes did not match up the call stack.
        // If now they match, it means that there is some error in the tree restore logic.
        // It's dangerous to continue running, so we shutdown.
        panic!("`find_hash_mismatch_point` didn't find the root hash divergence after scanning though all the blocks");
    }

    /// Loads the most recent state and updates the current state to match it.
    async fn apply_state_diff(&mut self, current_block: BlockNumber, diff: AccountUpdates) {
        // Committed state contains the latest state, every account in it already has everything from `diff` applied.
        // We could've just go though all the accounts in the committed state and insert it to the tree to achieve the same
        // result. However, it would be less efficient, as we would have to do it even for accounts that did not change,
        // and assuming that the tree is big enough, it's a lot of extra work.
        //
        // Instead, we are looking at the accounts that *actually* changed at least once and insert them to the tree only.
        // This way, we obtain the most recent state in a efficient way.
        let committed_state = self.storage.load_committed_state(current_block).await;

        // List of account IDs that were changed at least once between the currently observed state and the committed state.
        // `sort_unstable` and `dedup` is needed to list each account exactly once.
        let mut updated_accounts = diff.into_iter().map(|(id, _)| id).collect::<Vec<_>>();
        updated_accounts.sort_unstable();
        updated_accounts.dedup();

        for idx in updated_accounts {
            // Normally, accounts should not disappear from the state, as we don't have a "remove account" operation.
            // However, we have a `Delete` account update type, so if there was an update applied to some account, and later
            // this account is not observed in the state at all, it is safe to assume that it was deleted from the state.
            //
            // If that's not the case and it's some bug, the root hash will not match.
            if let Some(acc) = committed_state.get(&idx).cloned() {
                self.insert_account(idx, acc);
            } else {
                self.remove_account(idx);
            }
        }
    }

    /// Checks that current root hash matches the hash from the database.
    /// Panics with provided message otherwise.
    async fn assert_calculated_root(&mut self, message: &str, current_block: BlockNumber) {
        let root_hash_from_tree = self.tree.root_hash();
        let root_hash_from_db = self.storage.load_block_hash_from_db(current_block).await;

        if root_hash_from_tree != root_hash_from_db {
            panic!(
                "{}. \n \
                 Block {}. \n \
                 Root hash from the cached tree: {} \n \
                 Root hash from the database: {}",
                message, current_block, root_hash_from_tree, root_hash_from_db
            );
        }
    }

    fn insert_account(&mut self, id: AccountId, acc: Account) {
        self.acc_id_by_addr.insert(acc.address, id);
        self.tree.insert(*id, acc);
    }

    fn remove_account(&mut self, id: AccountId) -> Option<Account> {
        if let Some(acc) = self.tree.remove(*id) {
            self.acc_id_by_addr.remove(&acc.address);
            Some(acc)
        } else {
            None
        }
    }
}
