use std::collections::{
    hash_map::{Entry, Values},
    HashMap,
};

use rust_decimal::Decimal;

use crate::ledger::{
    Account, DisputeStatus, DisputeTransaction, DisputeTransactionType, Ledger,
    StandardTransaction, StandardTransactionType, Transaction,
};

/// In-memory implementation of a ledger which records transactions and
/// tracks account balances
pub(crate) struct HashMapLedger {
    transactions_by_id: HashMap<u32, StandardTransaction>,
    accounts_by_client_id: HashMap<u16, Account>,
}

impl HashMapLedger {
    pub fn new() -> HashMapLedger {
        HashMapLedger {
            transactions_by_id: HashMap::new(),
            accounts_by_client_id: HashMap::new(),
        }
    }

    fn handle_standard(&mut self, transaction: StandardTransaction) -> Result<(), String> {
        if transaction.amount <= Decimal::ZERO {
            return Err("Amount cannot be negative".into());
        }

        let account = self
            .accounts_by_client_id
            .entry(transaction.client_id)
            .or_insert_with(|| Account::new(transaction.client_id));

        // If the account is locked, we don't allow deposits and withdrawals.
        if account.is_locked {
            return Err(format!(
                "Account is locked for client id: {}",
                transaction.client_id
            ));
        }

        // If it's a withdrawal, ensure there are sufficient funds available
        if transaction.tx_type == StandardTransactionType::Withdrawal
            && transaction.amount > account.available
        {
            return Err("Insufficient funds available to process withdrawal".into());
        }

        account.available = match transaction.tx_type {
            StandardTransactionType::Deposit => account.available + transaction.amount,
            StandardTransactionType::Withdrawal => account.available - transaction.amount,
        };

        match self.transactions_by_id.entry(transaction.tx_id) {
            Entry::Occupied(entry) => Err(format!("Duplicate transaction id: {}", entry.key())),
            Entry::Vacant(entry) => Ok(entry.insert(transaction)),
        }?;

        Ok(())
    }

    fn handle_dispute(&mut self, transaction: DisputeTransaction) -> Result<(), String> {
        // Note: Disputes are still allowed for locked accounts, so we don't need to check the
        // 'is_locked' field.
        let account = self
            .accounts_by_client_id
            .get_mut(&transaction.client_id)
            .ok_or_else(|| format!("No account found with client id: {}", transaction.client_id))?;

        let transaction_for_dispute = self
            .transactions_by_id
            .get_mut(&transaction.tx_id)
            .ok_or_else(|| format!("No transaction found with id: {}", transaction.tx_id))?;

        // The spec doesn't explicitly say this, but it's assumed that specified client_id on the dispute
        // entry must match the client_id on the transaction being disputed.
        if transaction.client_id != transaction_for_dispute.client_id {
            return Err(format!(
                "Transaction with id {} does not belong to client {}",
                transaction.tx_id, transaction.client_id
            ));
        }

        match transaction_for_dispute.tx_type {
            StandardTransactionType::Withdrawal => {
                // According to the spec, when a transaction is disputed, the funds are moved
                // from 'available' to 'held'. So it really only makes sense to dispute deposits.
                // See README for more info on this assumption.
                return Err("Cannot dispute withdrawals".into());
            }
            StandardTransactionType::Deposit => {
                // Disputing a deposit is allowed; do nothing here.
            }
        }

        Ok(match transaction.tx_type {
            DisputeTransactionType::Dispute => {
                // Currently it's only possible for a single (unresolved) dispute to be raised
                // per transaction
                if transaction_for_dispute.dispute_status.is_some() {
                    return Err("Transaction already disputed".into());
                }

                transaction_for_dispute.dispute_status = Some(DisputeStatus::Unresolved);

                // Move funds from 'available' to 'held'.
                // Allow available funds to go into negative here. This represents
                // the scenario when funds have already been withdrawn before a dispute has been
                // raised. If this were to happen, it is assumed that the entity managing
                // the account would be liable for funding any resulting chargeback.
                // If a chargeback where to occur, the client account available and total
                // funds would remain in deficit.
                account.available -= transaction_for_dispute.amount;
                account.held += transaction_for_dispute.amount;
            }
            DisputeTransactionType::Resolve => {
                if let Some(dispute_status) = transaction_for_dispute.dispute_status {
                    match dispute_status {
                        DisputeStatus::Unresolved => {
                            // Do nothing -- this is the only case where resolving makes sense.
                        }
                        DisputeStatus::Chargeback => {
                            return Err("Transaction already charged back, cannot resolve".into());
                        }
                    }
                } else {
                    return Err("Transaction not disputed, cannot resolve".into());
                }

                // Clear the dispute_status and restore the funds from held to available.
                // Note it's possible that another dispute will be raised later.
                transaction_for_dispute.dispute_status = None;
                account.available += transaction_for_dispute.amount;
                account.held -= transaction_for_dispute.amount;
            }
            DisputeTransactionType::Chargeback => {
                if let Some(dispute_status) = transaction_for_dispute.dispute_status {
                    match dispute_status {
                        DisputeStatus::Unresolved => {
                            // Do nothing -- this is the only case where chargeback makes sense.
                        }
                        DisputeStatus::Chargeback => {
                            return Err(
                                "Transaction already charged back, cannot chargeback".into()
                            );
                        }
                    }
                } else {
                    return Err("Transaction not disputed, cannot chargeback".into());
                }

                // Withdraw the funds from held and lock the account.
                transaction_for_dispute.dispute_status = Some(DisputeStatus::Chargeback);
                account.held -= transaction_for_dispute.amount;
                account.is_locked = true;
            }
        })
    }
}

impl<'a> Ledger<'a> for HashMapLedger {
    type AccountsIterator = Values<'a, u16, Account>;
    type TransactionError = String;

    fn get_accounts(&'a self) -> Self::AccountsIterator {
        self.accounts_by_client_id.values()
    }

    fn handle_transaction(
        &mut self,
        transaction: Transaction,
    ) -> Result<(), Self::TransactionError> {
        match transaction {
            Transaction::Standard(standard_transaction) => {
                self.handle_standard(standard_transaction)
            }
            Transaction::Dispute(dispute_transaction) => self.handle_dispute(dispute_transaction),
        }
    }
}
