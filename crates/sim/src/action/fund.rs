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

use ethers::types::{Address, U256};
use rundler_provider::Provider;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Funder<P> {
    provider: Arc<P>,
}

impl<P: Provider> Funder<P> {
    pub fn new(provider: Arc<P>) -> Self {
        Self {
            provider: provider.clone(),
        }
    }
    pub async fn create_wallet(&self, vec: Vec<Address>, nonce: U256) -> anyhow::Result<()> {
        self.provider.create_wallet(vec, nonce)
    }

    // pub async fn charge_wallet(&self) -> anyhow::Result<()> {}
}