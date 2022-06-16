use std::{error::Error, io};

use clap::Parser;
use ledger::Ledger;

mod csv_accounts;
mod csv_transactions;
mod hashmap_ledger;
mod ledger;

#[derive(Parser, Default, Debug)]
#[clap(author = "Andrew Harward", about = "Example payments engine")]
struct Args {
    #[clap(forbid_empty_values = true, help = "Path to transactions CSV file")]
    transactions_csv_path: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let mut transactions_reader = csv_transactions::Reader::from_path(args.transactions_csv_path)?;

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
