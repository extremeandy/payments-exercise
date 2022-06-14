use std::{env, error::Error, io};

use ledger::Ledger;

mod csv_accounts;
mod csv_transactions;
mod hashmap_ledger;
mod ledger;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let transactions_csv_path = args
        .get(1)
        .ok_or("You must provide a file path as the first and only argument")?;

    let mut transactions_reader = csv_transactions::Reader::from_path(transactions_csv_path)?;

    let mut ledger = hashmap_ledger::HashMapLedger::new();

    for transaction in transactions_reader.iter() {
        // Note: Swallow *all* kinds of handling errors and continue - e.g. failed withdrawals,
        // duplicate transaction ids. Perhaps in future we would want to swallow only
        // some kinds of errors, and panic on others.
        let _ = ledger.handle_transaction(transaction?);
    }

    let accounts_writer = csv_accounts::Writer::from_writer(io::stdout());
    accounts_writer.write_all(ledger.get_accounts())?;

    Ok(())
}
