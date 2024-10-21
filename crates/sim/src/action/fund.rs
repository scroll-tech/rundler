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

use ethers::types::{Address, Bytes, U256};
use ethers::utils::parse_ether;
use futures_util::TryFutureExt;
use rundler_provider::Provider;
use rundler_types::contracts::v0_6::scroll_smart_wallet_factory::ScrollSmartWalletFactory;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Funder<P> {
    provider: Arc<P>,
    factory_address: Address,
}

impl<P: Provider> Funder<P> {
    pub fn new(provider: Arc<P>, factory_address: Address) -> Self {
        Self {
            provider: provider.clone(),
            factory_address,
        }
    }

    pub async fn create_wallet(&self, owners: Vec<Bytes>, nonce: U256) -> anyhow::Result<()> {
        let address = self.get_address(owners.clone(), nonce).await?;
        let balance = self.get_balance(address).await?;
        let required_balance = parse_ether(0.01)?;
        if balance < required_balance {
            return Err(anyhow::anyhow!(
                "Insufficient balance: wallet balance is {:?}, required at least {:?}",
                balance,
                required_balance
            ));
        }

        // TODO: add usdt/usdc

        self.create_account(owners, nonce).await?;
        Ok(())
    }

    async fn get_address(&self, owners: Vec<Bytes>, nonce: U256) -> anyhow::Result<Address> {
        let smart_wallet_factory = self.get_smart_wallet_factory();
        let wallet_address = smart_wallet_factory
            .get_address(owners, nonce)
            .call()
            .await
            .map_err(|err| anyhow::anyhow!("Failed to get address: {:?}", err))?;
        Ok(wallet_address)
    }

    async fn create_account(&self, owners: Vec<Bytes>, nonce: U256) -> anyhow::Result<Address> {
        let smart_wallet_factory = self.get_smart_wallet_factory();
        let wallet_address = smart_wallet_factory
            .create_account(owners, nonce)
            .call()
            .await
            .map_err(|err| anyhow::anyhow!("Failed to create account: {:?}", err))?;
        Ok(wallet_address)
    }

    async fn get_balance(&self, address: Address) -> anyhow::Result<U256> {
        let balance = self
            .provider
            .get_balance(address, None)
            .await
            .map_err(|err| {
                anyhow::anyhow!("Failed to get balance of {:?}, err: {:?}", address, err)
            })?;
        Ok(balance)
    }

    // 实例化 ScrollSmartWalletFactory
    fn get_smart_wallet_factory(&self) -> ScrollSmartWalletFactory<Arc<dyn Provider>> {
        ScrollSmartWalletFactory::new(self.factory_address, Arc::clone(self.provider.as_ref()))
    }
}
