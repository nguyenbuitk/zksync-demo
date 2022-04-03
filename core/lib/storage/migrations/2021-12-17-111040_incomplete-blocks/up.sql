-- Incomplete block (one that does not yet have a root hash calculated) header entry.
-- It mimics the `block` table with the only exception that it does not have the block root hash in it.
-- Commitment is also not included, since it relies on the root hash of the block.
CREATE TABLE IF NOT EXISTS incomplete_blocks (
    number BIGINT PRIMARY KEY,
    fee_account_id BIGINT NOT NULL,
    unprocessed_prior_op_before BIGINT NOT NULL,
    unprocessed_prior_op_after BIGINT NOT NULL,
    block_size BIGINT NOT NULL,
    commit_gas_limit BIGINT NOT NULL,
    verify_gas_limit BIGINT NOT NULL,
    timestamp bigint
);

-- Entry in `block_metadata` is created before corresponding entry in `blocks` table.
ALTER TABLE block_metadata DROP CONSTRAINT block_metadata_block_number_fkey;
