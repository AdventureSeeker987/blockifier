use std::collections::BTreeMap;

use blockifier::transaction::account_transaction::AccountTransaction;
use blockifier::transaction::transaction_execution::Transaction;
use blockifier::transaction::transaction_types::TransactionType;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use starknet_api::transaction::{Resource, ResourceBounds};
use starknet_api::StarknetApiError;

use crate::errors::{NativeBlockifierInputError, NativeBlockifierResult};
use crate::py_declare::py_declare;
use crate::py_deploy_account::py_deploy_account;
use crate::py_invoke_function::py_invoke_function;
use crate::py_l1_handler::py_l1_handler;

// Structs.

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
pub enum PyResource {
    L1Gas,
    L2Gas,
}

impl From<PyResource> for starknet_api::transaction::Resource {
    fn from(py_resource: PyResource) -> Self {
        match py_resource {
            PyResource::L1Gas => starknet_api::transaction::Resource::L1Gas,
            PyResource::L2Gas => starknet_api::transaction::Resource::L2Gas,
        }
    }
}

impl FromPyObject<'_> for PyResource {
    fn extract(resource: &PyAny) -> PyResult<Self> {
        let resource_name: &str = resource.getattr("name")?.extract()?;
        match resource_name {
            "L1_GAS" => Ok(PyResource::L1Gas),
            "L2_GAS" => Ok(PyResource::L2Gas),
            _ => Err(PyValueError::new_err(format!("Invalid resource: {resource_name}"))),
        }
    }
}

#[derive(Clone, Copy, Default, FromPyObject)]
pub struct PyResourceBounds {
    pub max_amount: u64,
    pub max_price_per_unit: u128,
}

impl From<PyResourceBounds> for starknet_api::transaction::ResourceBounds {
    fn from(py_resource_bounds: PyResourceBounds) -> Self {
        Self {
            max_amount: py_resource_bounds.max_amount,
            max_price_per_unit: py_resource_bounds.max_price_per_unit,
        }
    }
}

#[derive(Clone, FromPyObject)]
pub struct PyResourceBoundsMapping(pub BTreeMap<PyResource, PyResourceBounds>);

impl TryFrom<PyResourceBoundsMapping> for starknet_api::transaction::ResourceBoundsMapping {
    type Error = StarknetApiError;
    fn try_from(py_resource_bounds_mapping: PyResourceBoundsMapping) -> Result<Self, Self::Error> {
        let resource_bounds_vec: Vec<(Resource, ResourceBounds)> = py_resource_bounds_mapping
            .0
            .into_iter()
            .map(|(py_resource_type, py_resource_bounds)| {
                (Resource::from(py_resource_type), ResourceBounds::from(py_resource_bounds))
            })
            .collect();
        Self::try_from(resource_bounds_vec)
    }
}

#[derive(Clone)]
pub enum PyDataAvailabilityMode {
    L1 = 0,
    L2 = 1,
}

impl FromPyObject<'_> for PyDataAvailabilityMode {
    fn extract(data_availability_mode: &PyAny) -> PyResult<Self> {
        let data_availability_mode: u8 = data_availability_mode.extract()?;
        match data_availability_mode {
            0 => Ok(PyDataAvailabilityMode::L1),
            1 => Ok(PyDataAvailabilityMode::L2),
            _ => Err(PyValueError::new_err(format!(
                "Invalid data availability mode: {data_availability_mode}"
            ))),
        }
    }
}

impl From<PyDataAvailabilityMode> for starknet_api::data_availability::DataAvailabilityMode {
    fn from(py_data_availability_mode: PyDataAvailabilityMode) -> Self {
        match py_data_availability_mode {
            PyDataAvailabilityMode::L1 => starknet_api::data_availability::DataAvailabilityMode::L1,
            PyDataAvailabilityMode::L2 => starknet_api::data_availability::DataAvailabilityMode::L2,
        }
    }
}

// Transaction creation.

pub fn py_account_tx(
    tx: &PyAny,
    raw_contract_class: Option<&str>,
) -> NativeBlockifierResult<AccountTransaction> {
    let Transaction::AccountTransaction(account_tx) = py_tx(tx, raw_contract_class)? else {
        panic!("Not an account transaction.");
    };

    Ok(account_tx)
}

pub fn py_tx(tx: &PyAny, raw_contract_class: Option<&str>) -> NativeBlockifierResult<Transaction> {
    let tx_type: &str = tx.getattr("tx_type")?.getattr("name")?.extract()?;
    let tx_type: TransactionType =
        tx_type.parse().map_err(NativeBlockifierInputError::ParseError)?;

    Ok(match tx_type {
        TransactionType::Declare => {
            let raw_contract_class: &str = raw_contract_class
                .expect("A contract class must be passed in a Declare transaction.");
            AccountTransaction::Declare(py_declare(tx, raw_contract_class)?).into()
        }
        TransactionType::DeployAccount => {
            AccountTransaction::DeployAccount(py_deploy_account(tx)?).into()
        }
        TransactionType::InvokeFunction => {
            AccountTransaction::Invoke(py_invoke_function(tx)?).into()
        }
        TransactionType::L1Handler => py_l1_handler(tx)?.into(),
    })
}
