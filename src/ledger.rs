use rust_decimal::Decimal;
use serde::Deserialize;

/// The idea of this trait is that there could be alternate implementations which
/// share common code for business logic. That hasn't really been fleshed out though
/// and would need a lot more thought.
pub(crate) trait Ledger<'a> {
    type AccountsIterator: Iterator<Item = &'a Account>;
    type TransactionError;

    fn get_accounts(&'a self) -> Self::AccountsIterator;

    fn handle_transaction(
        &mut self,
        transaction: Transaction,
    ) -> Result<(), Self::TransactionError>;
}

pub(crate) struct Account {
    pub client_id: u16,
    pub available: Decimal,
    pub held: Decimal,
    pub is_locked: bool,
}

impl Account {
    pub fn new(client_id: u16) -> Account {
        Account {
            client_id: client_id,
            available: Decimal::default(),
            held: Decimal::default(),
            is_locked: false,
        }
    }

    pub fn total(&self) -> Decimal {
        self.available + self.held
    }
}

#[derive(Debug)]
pub(crate) enum Transaction {
    Standard(StandardTransaction),
    Dispute(DisputeTransaction),
}

/// 'Standard' transaction here means either a deposit or a withdrawal
#[derive(Debug)]
pub(crate) struct StandardTransaction {
    pub tx_type: StandardTransactionType,
    pub client_id: u16,
    pub tx_id: u32,
    pub amount: Decimal,
    pub dispute_status: Option<DisputeStatus>,
}

/// 'Standard' transaction here means either a deposit or a withdrawal
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
pub(crate) enum StandardTransactionType {
    Deposit,
    Withdrawal,
}

#[derive(Debug)]
pub(crate) struct DisputeTransaction {
    pub tx_type: DisputeTransactionType,
    pub client_id: u16,
    pub tx_id: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum DisputeTransactionType {
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum DisputeStatus {
    Unresolved,
    Chargeback,
}
