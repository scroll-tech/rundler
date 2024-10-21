// This file is part of Rundler.
//
// Rundler is free software: you can redistribute it and/or modify it under the
// terms of the GNU Lesser General Public License as published by the Free Software
// Foundation, either version 3 of the License, or (at your option) any later version.
//
// Rundler is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
// See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with Rundler.
// If not, see https://www.gnu.org/licenses/.

use crate::{
    types::RpcScrollCreateWallet,
    utils::{self, InternalRpcResult},
};
use anyhow::Context;
use async_trait::async_trait;
use ethers::types::Address;
use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use rundler_provider::Provider;
use rundler_sim::Funder;
use rundler_types::pool::Pool;
use std::sync::Arc;

/// Scroll API
#[rpc(client, server, namespace = "scroll")]
pub trait ScrollApi {
    /// Creates the wallet for the user if meet conditions
    #[method(name = "createWallet")]
    async fn create_wallet(&self, create_wallet_params: RpcScrollCreateWallet) -> RpcResult<String>;
}

pub(crate) struct ScrollApi<P> {
    funder: Funder<P>,
}

#[async_trait]
impl<P> ScrollApiServer for ScrollApi<P>
where
    P: Provider,
{
    async fn create_wallet(&self, create_wallet_params: RpcScrollCreateWallet) -> RpcResult<String> {
        utils::safe_call_rpc_handler(
            "scroll_createWallet",
            ScrollApi::create_wallet(self, create_wallet_params),
        ).await
    }
}

impl<P> ScrollApi<P>
where
    P: Provider,
{
    pub(crate) fn new(provider: Arc<P>) -> Self {
        Self {
            funder: Funder::new(provider),
        }
    }

    async fn create_wallet(&self, clear_params: RpcScrollCreateWallet) -> InternalRpcResult<String> {
        self.funder.
            create_wallet(
                clear_params.owners, clear_params.nonce,
            )
            .await
            .context("should create wallet")?;

        Ok("ok".to_string())
    }
}