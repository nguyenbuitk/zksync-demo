pub use zksync_types::EthBlockId;
use zksync_types::{tx::TxEthSignature, SignedZkSyncTx};

use crate::tx_error::TxAddError;

/// `CoreApiClient` is capable of interacting with a private zkSync Core API.
#[derive(Debug, Clone)]
pub struct CoreApiClient {
    client: reqwest::Client,
    addr: String,
}

impl CoreApiClient {
    pub fn new(addr: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            addr,
        }
    }

    /// Sends a new transaction to the Core mempool.
    pub async fn send_tx(&self, tx: SignedZkSyncTx) -> anyhow::Result<Result<(), TxAddError>> {
        let endpoint = format!("{}/new_tx", self.addr);
        self.post(&endpoint, tx).await
    }

    /// Sends a new transactions batch to the Core mempool.
    pub async fn send_txs_batch(
        &self,
        txs: Vec<SignedZkSyncTx>,
        eth_signatures: Vec<TxEthSignature>,
    ) -> anyhow::Result<Result<(), TxAddError>> {
        let endpoint = format!("{}/new_txs_batch", self.addr);
        let data = (txs, eth_signatures);

        self.post(&endpoint, data).await
    }

    async fn post<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        request: impl serde::Serialize,
    ) -> anyhow::Result<T> {
        let response = self
            .client
            .post(url)
            .json(&request)
            .send()
            .await?
            .json()
            .await?;

        Ok(response)
    }
}
