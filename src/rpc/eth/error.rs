use ethers::types::{Address, U256};
use jsonrpsee::{
    core::Error as RpcError,
    types::{
        error::{CallError, INTERNAL_ERROR_CODE, INVALID_PARAMS_CODE},
        ErrorObject,
    },
};
use serde::Serialize;

// Error codes borrowed from jsonrpsee
// INVALID_PARAMS_CODE = -32602
// INTERNAL_ERROR_CODE = -32603

// Custom ERC-4337 error codes
const ENTRYPOINT_VALIDATION_REJECTED_CODE: i32 = -32500;
const PAYMASTER_VALIDATION_REJECTED_CODE: i32 = -32501;
const OPCODE_VIOLATION_CODE: i32 = -32502;
const OUT_OF_TIME_RANGE_CODE: i32 = -32503;
const THROTTLED_OR_BANNED_CODE: i32 = -32504;
const STAKE_TOO_LOW_CODE: i32 = -32505;
const UNSUPORTED_AGGREGATOR_CODE: i32 = -32506;
const SIGNATURE_CHECK_FAILED_CODE: i32 = -32507;

/// Error returned by the RPC server eth namespace
#[derive(Debug, thiserror::Error)]
pub enum EthRpcError {
    /// Invalid parameters
    #[error("invalid parameters: {0:?}")]
    InvalidParams(String),
    /// Validation rejected the operation in entrypoint or during
    /// wallet creation or validation
    #[error("{0}")]
    EntrypointValidationRejected(String),
    /// Paymaster rejected the operation
    #[error("{}", .0.reason)]
    PaymasterValidatoinRejected(PaymasterValidationRejectedData),
    /// Opcode violation
    #[error("opcode violation: {0}")]
    OpcodeViolation(String),
    /// Operation is out of time range
    #[error("operation is out of time range")]
    OutOfTimeRange(OutOfTimeRangeData),
    /// Entity throttled or banned
    #[error("entity throttled or banned")]
    ThrottledOrBanned(ThrottledOrBannedData),
    /// Entity stake/unstake delay too low
    #[error("entity stake/unstake delay too low")]
    StakeTooLow(StakeTooLowData),
    /// Unsupported aggregator
    #[error("unsupported aggregator")]
    UnsupportedAggregator(UnsupportedAggregatorData),
    /// Other internal errors
    #[error("signature check failed")]
    SignatureCheckFailed,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct PaymasterValidationRejectedData {
    paymaster: Address,
    #[serde(skip_serializing)] // this is included in the message
    reason: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutOfTimeRangeData {
    valid_until: U256,
    valid_after: U256,
    paymaster: Option<Address>,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct ThrottledOrBannedData {
    paymaster: Option<Address>,
    aggregator: Option<Address>,
    factory: Option<Address>,
}

impl ThrottledOrBannedData {
    pub fn paymaster(paymaster: Address) -> Self {
        Self {
            paymaster: Some(paymaster),
            aggregator: None,
            factory: None,
        }
    }

    pub fn aggregator(aggregator: Address) -> Self {
        Self {
            paymaster: None,
            aggregator: Some(aggregator),
            factory: None,
        }
    }

    pub fn factory(factory: Address) -> Self {
        Self {
            paymaster: None,
            aggregator: None,
            factory: Some(factory),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StakeTooLowData {
    paymaster: Option<Address>,
    aggregator: Option<Address>,
    factory: Option<Address>,
    minimum_stake: U256,
    minimum_unstake_delay: U256,
}

impl StakeTooLowData {
    pub fn paymaster(paymaster: Address, minimum_stake: U256, minimum_unstake_delay: U256) -> Self {
        Self {
            paymaster: Some(paymaster),
            aggregator: None,
            factory: None,
            minimum_stake,
            minimum_unstake_delay,
        }
    }

    pub fn aggregator(
        aggregator: Address,
        minimum_stake: U256,
        minimum_unstake_delay: U256,
    ) -> Self {
        Self {
            paymaster: None,
            aggregator: Some(aggregator),
            factory: None,
            minimum_stake,
            minimum_unstake_delay,
        }
    }

    pub fn factory(factory: Address, minimum_stake: U256, minimum_unstake_delay: U256) -> Self {
        Self {
            paymaster: None,
            aggregator: None,
            factory: Some(factory),
            minimum_stake,
            minimum_unstake_delay,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct UnsupportedAggregatorData {
    aggregator: Address,
}

impl From<EthRpcError> for RpcError {
    fn from(error: EthRpcError) -> Self {
        match error {
            EthRpcError::InvalidParams(msg) => rpc_err(INVALID_PARAMS_CODE, msg),
            EthRpcError::ValidationRejected(_) => {
                rpc_err(VALIDATION_REJECTED_CODE, error.to_string())
            }
            EthRpcError::PaymasterRejected(data) => {
                rpc_err_with_data(PAYMASTER_REJECTED_CODE, error.to_string(), data)
            }
            EthRpcError::OpcodeViolation(_) => rpc_err(OPCODE_VIOLATION_CODE, error.to_string()),
            EthRpcError::OutOfTimeRange(data) => {
                rpc_err_with_data(OUT_OF_TIME_RANGE_CODE, error.to_string(), data)
            }
            EthRpcError::ThrottledOrBanned(data) => {
                rpc_err_with_data(THROTTLED_OR_BANNED_CODE, error.to_string(), data)
            }
            EthRpcError::StakeTooLow(data) => {
                rpc_err_with_data(STAKE_TOO_LOW_CODE, error.to_string(), data)
            }
            EthRpcError::UnsupportedAggregator(data) => {
                rpc_err_with_data(UNSUPORTED_AGGREGATOR_CODE, error.to_string(), data)
            }
            EthRpcError::SignatureCheckFailed => {
                rpc_err(SIGNATURE_CHECK_FAILED_CODE, error.to_string())
            }
            EthRpcError::Internal(e) => rpc_err(INTERNAL_ERROR_CODE, e.to_string()),
        }
    }
}

fn rpc_err(code: i32, msg: impl Into<String>) -> RpcError {
    create_rpc_err(code, msg, None::<()>)
}

fn rpc_err_with_data<S: Serialize>(code: i32, msg: impl Into<String>, data: S) -> RpcError {
    create_rpc_err(code, msg, Some(data))
}

fn create_rpc_err<S: Serialize>(code: i32, msg: impl Into<String>, data: Option<S>) -> RpcError {
    RpcError::Call(CallError::Custom(ErrorObject::owned(
        code,
        msg.into(),
        data,
    )))
}