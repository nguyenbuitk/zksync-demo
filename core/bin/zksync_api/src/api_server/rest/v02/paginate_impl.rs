// Built-in uses

// External uses

// Workspace uses
use zksync_api_types::{
    v02::{
        block::BlockInfo,
        pagination::{
            AccountTxsRequest, ApiEither, BlockAndTxHash, Paginated, PaginationQuery,
            PendingOpsRequest,
        },
        transaction::{Transaction, TxHashSerializeWrapper},
    },
    Either,
};
use zksync_storage::StorageProcessor;
use zksync_types::{BlockNumber, SerialId, Token, TokenId};

// Local uses
use super::{
    block::block_info_from_details,
    error::{Error, InvalidDataError},
    paginate_trait::Paginate,
};

use zksync_api_types::v02::transaction::{L1Transaction, TransactionData, TxInBlockStatus};

#[async_trait::async_trait]
impl Paginate<ApiEither<TokenId>> for StorageProcessor<'_> {
    type OutputObj = Token;
    type OutputId = TokenId;

    async fn paginate(
        &mut self,
        query: &PaginationQuery<ApiEither<TokenId>>,
    ) -> Result<Paginated<Token, TokenId>, Error> {
        let mut transaction = self.start_transaction().await.map_err(Error::storage)?;

        let token_id = match query.from.inner {
            Either::Left(token_id) => token_id,
            Either::Right(_) => TokenId(
                transaction
                    .tokens_schema()
                    .get_max_erc20_token_id()
                    .await
                    .map_err(Error::storage)?,
            ),
        };

        let query = PaginationQuery {
            from: token_id,
            limit: query.limit,
            direction: query.direction,
        };

        let tokens = transaction
            .tokens_schema()
            .load_token_page(&query)
            .await
            .map_err(Error::storage)?;
        let count = transaction
            .tokens_schema()
            .get_count()
            .await
            .map_err(Error::storage)?;
        transaction.commit().await.map_err(Error::storage)?;

        Ok(Paginated::new(
            tokens,
            query.from,
            query.limit,
            query.direction,
            count,
        ))
    }
}

#[async_trait::async_trait]
impl Paginate<ApiEither<BlockNumber>> for StorageProcessor<'_> {
    type OutputObj = BlockInfo;
    type OutputId = BlockNumber;

    async fn paginate(
        &mut self,
        query: &PaginationQuery<ApiEither<BlockNumber>>,
    ) -> Result<Paginated<BlockInfo, BlockNumber>, Error> {
        let mut transaction = self.start_transaction().await.map_err(Error::storage)?;

        let last_block = transaction
            .chain()
            .block_schema()
            .get_last_committed_confirmed_block()
            .await
            .map_err(Error::storage)?;

        let block_number = match query.from.inner {
            Either::Left(block_number) => block_number,
            Either::Right(_) => last_block,
        };

        let query = PaginationQuery {
            from: block_number,
            limit: query.limit,
            direction: query.direction,
        };

        let blocks = transaction
            .chain()
            .block_schema()
            .load_block_page(&query)
            .await
            .map_err(Error::storage)?;
        let blocks: Vec<BlockInfo> = blocks.into_iter().map(block_info_from_details).collect();

        transaction.commit().await.map_err(Error::storage)?;

        Ok(Paginated::new(
            blocks,
            query.from,
            query.limit,
            query.direction,
            *last_block,
        ))
    }
}

#[async_trait::async_trait]
impl Paginate<BlockAndTxHash> for StorageProcessor<'_> {
    type OutputObj = Transaction;
    type OutputId = TxHashSerializeWrapper;

    async fn paginate(
        &mut self,
        query: &PaginationQuery<BlockAndTxHash>,
    ) -> Result<Paginated<Transaction, TxHashSerializeWrapper>, Error> {
        let mut transaction = self.start_transaction().await.map_err(Error::storage)?;

        let tx_hash = match query.from.tx_hash.inner {
            Either::Left(tx_hash) => tx_hash,
            Either::Right(_) => {
                if let Some(tx_hash) = transaction
                    .chain()
                    .operations_ext_schema()
                    .get_block_last_tx_hash(query.from.block_number)
                    .await
                    .map_err(Error::storage)?
                {
                    tx_hash
                } else {
                    return Ok(Paginated::new(
                        Vec::new(),
                        Default::default(),
                        query.limit,
                        query.direction,
                        0,
                    ));
                }
            }
        };

        let query = PaginationQuery {
            from: BlockAndTxHash {
                block_number: query.from.block_number,
                tx_hash: ApiEither::from(tx_hash),
            },
            limit: query.limit,
            direction: query.direction,
        };

        let txs = transaction
            .chain()
            .block_schema()
            .get_block_transactions_page(&query)
            .await
            .map_err(Error::storage)?
            .ok_or_else(|| Error::from(InvalidDataError::TransactionNotFound))?;
        let count = transaction
            .chain()
            .block_schema()
            .get_block_transactions_count(query.from.block_number)
            .await
            .map_err(Error::storage)?;

        transaction.commit().await.map_err(Error::storage)?;

        Ok(Paginated::new(
            txs,
            TxHashSerializeWrapper(tx_hash),
            query.limit,
            query.direction,
            count,
        ))
    }
}

#[async_trait::async_trait]
impl Paginate<AccountTxsRequest> for StorageProcessor<'_> {
    type OutputObj = Transaction;
    type OutputId = TxHashSerializeWrapper;

    async fn paginate(
        &mut self,
        query: &PaginationQuery<AccountTxsRequest>,
    ) -> Result<Paginated<Transaction, TxHashSerializeWrapper>, Error> {
        let mut transaction = self.start_transaction().await.map_err(Error::storage)?;

        let tx_hash = match query.from.tx_hash.inner {
            Either::Left(tx_hash) => tx_hash,
            Either::Right(_) => {
                if let Some(tx_hash) = transaction
                    .chain()
                    .operations_ext_schema()
                    .get_account_last_tx_hash(query.from.address)
                    .await
                    .map_err(Error::storage)?
                {
                    tx_hash
                } else {
                    return Ok(Paginated::new(
                        Vec::new(),
                        Default::default(),
                        query.limit,
                        query.direction,
                        0,
                    ));
                }
            }
        };

        let query = PaginationQuery {
            from: AccountTxsRequest {
                tx_hash: ApiEither::from(tx_hash),
                ..query.from
            },
            limit: query.limit,
            direction: query.direction,
        };

        let txs = transaction
            .chain()
            .operations_ext_schema()
            .get_account_transactions(&query)
            .await
            .map_err(Error::storage)?
            .ok_or_else(|| Error::from(InvalidDataError::TransactionNotFound))?;
        let count = transaction
            .chain()
            .operations_ext_schema()
            .get_account_transactions_count(
                query.from.address,
                query.from.token,
                query.from.second_address,
            )
            .await
            .map_err(Error::storage)?;

        transaction.commit().await.map_err(Error::storage)?;

        Ok(Paginated::new(
            txs,
            TxHashSerializeWrapper(tx_hash),
            query.limit,
            query.direction,
            count,
        ))
    }
}

#[async_trait::async_trait]
impl Paginate<PendingOpsRequest> for StorageProcessor<'_> {
    type OutputObj = Transaction;
    type OutputId = SerialId;

    async fn paginate(
        &mut self,
        query: &PaginationQuery<PendingOpsRequest>,
    ) -> Result<Paginated<Transaction, Self::OutputId>, Error> {
        let serial_id = match query.from.serial_id.inner {
            Either::Left(serial_id) => serial_id,
            // Right means the latest serial id
            Either::Right(_) => {
                if let Some(serial_id) = self
                    .chain()
                    .mempool_schema()
                    .get_max_serial_id_pending_deposits(query.from.address)
                    .await?
                {
                    serial_id
                } else {
                    return Ok(Paginated::new(
                        Vec::new(),
                        Default::default(),
                        query.limit,
                        query.direction,
                        0,
                    ));
                }
            }
        };
        let result = self
            .chain()
            .mempool_schema()
            .get_pending_deposits_for(query.from.address, serial_id, query.limit, query.direction)
            .await
            .map_err(Error::storage)?;

        let count = result.len() as u32;
        let txs = result
            .into_iter()
            .map(|op| {
                let tx_hash = op.tx_hash();
                let tx = L1Transaction::from_pending_op(
                    op.data.clone(),
                    op.eth_hash,
                    op.serial_id,
                    tx_hash,
                );
                Transaction {
                    tx_hash,
                    block_number: None,
                    op: TransactionData::L1(tx),
                    status: TxInBlockStatus::Queued,
                    fail_reason: None,
                    created_at: None,
                    batch_id: None,
                }
            })
            .collect();

        Ok(Paginated::new(
            txs,
            serial_id,
            query.limit,
            query.direction,
            count,
        ))
    }
}
