use std::fs::File;
use std::{fmt, path::Path};

use csv::{DeserializeRecordsIter, ReaderBuilder};
use rust_decimal::Decimal;
use serde::Deserialize;

use crate::ledger::{
    DisputeTransaction, DisputeTransactionType, StandardTransaction, StandardTransactionType,
    Transaction,
};

pub(crate) struct Reader(csv::Reader<File>);

impl Reader {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Reader, Error> {
        let underlying_reader = ReaderBuilder::new()
            .trim(csv::Trim::All)
            .from_path(path)
            .map_err(|err| Error::Csv(err))?;

        Ok(Reader(underlying_reader))
    }

    pub fn iter<'a>(&mut self) -> CsvTransactionIterator {
        CsvTransactionIterator(self.0.deserialize())
    }
}

pub(crate) struct CsvTransactionIterator<'r>(DeserializeRecordsIter<'r, File, TransactionRecord>);

impl<'a> Iterator for CsvTransactionIterator<'a> {
    type Item = Result<Transaction, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let next_result = self.0.next()?;
        Some(
            next_result
                .map_err(Error::Csv)
                .and_then(|r| r.try_into().map_err(Error::InvalidTransaction)),
        )
    }
}

impl TryFrom<TransactionRecord> for Transaction {
    type Error = InvalidTransactionError;

    fn try_from(record: TransactionRecord) -> Result<Self, Self::Error> {
        let transaction = match record.tx_type.into() {
            TransactionType::Standard(tx_type) => Transaction::Standard(StandardTransaction {
                tx_type: tx_type,
                client_id: record.client_id,
                tx_id: record.tx_id,
                amount: record
                    .amount
                    .ok_or(InvalidTransactionError::AmountNotSpecified)?,
                dispute_status: None,
            }),
            TransactionType::Dispute(tx_type) => {
                if record.amount.is_some() {
                    return Err(InvalidTransactionError::AmountUnexpectedForDispute);
                }

                Transaction::Dispute(DisputeTransaction {
                    tx_type: tx_type,
                    client_id: record.client_id,
                    tx_id: record.tx_id,
                })
            }
        };

        Ok(transaction)
    }
}

/// CSV-serializable version of a transaction
#[derive(Debug, Deserialize)]
struct TransactionRecord {
    #[serde(rename = "type")]
    tx_type: CsvTransactionType,

    #[serde(rename = "client")]
    client_id: u16,

    #[serde(rename = "tx")]
    tx_id: u32,

    amount: Option<Decimal>,
}

/// This is a temporary type that is used to simplify conversion from
/// [`TransactionCsvRecord`] to [`Transaction`].
enum TransactionType {
    Standard(StandardTransactionType),
    Dispute(DisputeTransactionType),
}

impl From<CsvTransactionType> for TransactionType {
    fn from(t: CsvTransactionType) -> Self {
        match t {
            CsvTransactionType::Deposit => {
                TransactionType::Standard(StandardTransactionType::Deposit)
            }
            CsvTransactionType::Withdrawal => {
                TransactionType::Standard(StandardTransactionType::Withdrawal)
            }
            CsvTransactionType::Dispute => {
                TransactionType::Dispute(DisputeTransactionType::Dispute)
            }
            CsvTransactionType::Resolve => {
                TransactionType::Dispute(DisputeTransactionType::Resolve)
            }
            CsvTransactionType::Chargeback => {
                TransactionType::Dispute(DisputeTransactionType::Chargeback)
            }
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")] // Note that currently only lowercase types are supported when deserializing
enum CsvTransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug)]
pub enum Error {
    Csv(csv::Error),
    InvalidTransaction(InvalidTransactionError),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Csv(err) => write!(f, "CSV error: {}", err),
            Self::InvalidTransaction(err) => write!(f, "Invalid transaction: {}", err),
        }
    }
}

#[derive(Debug)]
pub enum InvalidTransactionError {
    AmountNotSpecified,
    AmountUnexpectedForDispute,
}

// TODO: Is this even used...? Not sure why but it doesn't seem to be used to format
// the error when printing to stderr.
impl fmt::Display for InvalidTransactionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AmountNotSpecified => {
                write!(f, "Amount not specified")
            }
            Self::AmountUnexpectedForDispute => {
                write!(
                    f,
                    "Amount should not be specified for dispute/resolve/chargeback transactions"
                )
            }
        }
    }
}
