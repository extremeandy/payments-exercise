use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::process::Command;

const BIN_NAME: &str = "payments-engine";

#[test]
fn command_fails_when_file_doesnt_exist() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin(BIN_NAME)?;

    cmd.arg("invalid-file.csv");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No such file or directory"));

    Ok(())
}

#[test]
fn command_fails_when_csv_incorrectly_formatted() -> Result<(), Box<dyn std::error::Error>> {
    let csv_file = assert_fs::NamedTempFile::new("transactions.csv")?;

    // Note that the deposit is missing a column for amount.
    csv_file.write_str(
        "type, client, tx, amount
deposit, 1, 1",
    )?;

    let mut cmd = Command::cargo_bin(BIN_NAME)?;
    cmd.arg(csv_file.path());

    cmd.assert()
        .failure()
        // TODO: Provide a more user-friendly error!
        .stderr(predicate::str::contains("UnequalLengths"));

    Ok(())
}

#[test]
fn command_fails_when_amount_missing_for_deposit() -> Result<(), Box<dyn std::error::Error>> {
    let csv_file = assert_fs::NamedTempFile::new("transactions.csv")?;

    // Note that the deposit is missing a column for amount.
    csv_file.write_str(
        "type, client, tx, amount
deposit, 1, 1,",
    )?;

    let mut cmd = Command::cargo_bin(BIN_NAME)?;
    cmd.arg(csv_file.path());

    cmd.assert()
        .failure()
        // TODO: Provide a more user-friendly error!
        .stderr(predicate::str::contains("AmountNotSpecified"));

    Ok(())
}

#[test]
fn command_fails_when_amount_present_for_dispute() -> Result<(), Box<dyn std::error::Error>> {
    let csv_file = assert_fs::NamedTempFile::new("transactions.csv")?;

    // Note that the deposit is missing a column for amount.
    csv_file.write_str(
        "type, client, tx, amount
deposit, 1, 1, 5.0
dispute, 1, 1, 3.0",
    )?;

    let mut cmd = Command::cargo_bin(BIN_NAME)?;
    cmd.arg(csv_file.path());

    cmd.assert().failure();

    Ok(())
}

#[test]
fn basic_example() -> Result<(), Box<dyn std::error::Error>> {
    let csv_content = "type, client, tx, amount
deposit, 1, 1, 1.0
deposit, 2, 2, 2.0
deposit, 1, 3, 2.0
withdrawal, 1, 4, 1.5
withdrawal, 2, 5, 3.0";

    let expected_rows = &mut ["1,1.5,0,1.5,false", "2,2,0,2,false"];

    assert_cmd_succeeds_with_result(csv_content, expected_rows)
}

#[test]
fn dispute() -> Result<(), Box<dyn std::error::Error>> {
    let csv_content = "type, client, tx, amount
deposit, 1, 1, 1.0
deposit, 2, 2, 2.0
deposit, 1, 3, 2.0
withdrawal, 1, 4, 1.5
withdrawal, 2, 5, 3.0
dispute, 1, 1,";

    let expected_rows = &mut ["1,0.5,1,1.5,false", "2,2,0,2,false"];

    assert_cmd_succeeds_with_result(csv_content, expected_rows)
}

#[test]
fn dispute_then_resolve() -> Result<(), Box<dyn std::error::Error>> {
    let csv_content = "type, client, tx, amount
deposit, 1, 1, 1.0
deposit, 2, 2, 2.0
deposit, 1, 3, 2.0
withdrawal, 1, 4, 1.5
withdrawal, 2, 5, 3.0
dispute, 1, 1,
withdrawal, 1, 6, 0.5
resolve, 1, 1,
withdrawal, 1, 7, 0.5";

    let expected_rows = &mut ["1,0.5,0,0.5,false", "2,2,0,2,false"];

    assert_cmd_succeeds_with_result(csv_content, expected_rows)
}

#[test]
fn multiple_disputes_for_single_client() -> Result<(), Box<dyn std::error::Error>> {
    let csv_content = "type, client, tx, amount
deposit, 1, 1, 1.0
deposit, 2, 2, 2.0
deposit, 1, 3, 2.0
dispute, 1, 1,
dispute, 1, 3,
withdrawal, 1, 4, 1.5
withdrawal, 2, 5, 3.0";

    let expected_rows = &mut ["1,0,3,3,false", "2,2,0,2,false"];

    assert_cmd_succeeds_with_result(csv_content, expected_rows)
}

/// In this case, funds have already been withdrawn, so the available balance goes
/// negative. It's assumed that the entity managing the account is liable for
/// funding chargebacks regardless of whether or not the client account contains
/// the funds to pay it. The client account would then be in deficit.
#[test]
fn dispute_funds_already_withdrawn() -> Result<(), Box<dyn std::error::Error>> {
    let csv_content = "type, client, tx, amount
deposit, 1, 1, 1.0
withdrawal, 1, 2, 0.5
dispute, 1, 1,";

    let expected_rows = &mut ["1,-0.5,1,0.5,false"];

    assert_cmd_succeeds_with_result(csv_content, expected_rows)
}

/// As for [`dispute_funds_already_withdrawn`], but with chargeback. Client
/// account ends up in deficit and locked.
#[test]
fn dispute_then_chargeback_funds_already_withdrawn() -> Result<(), Box<dyn std::error::Error>> {
    let csv_content = "type, client, tx, amount
deposit, 1, 1, 1.0
withdrawal, 1, 2, 0.5
dispute, 1, 1,
chargeback, 1, 1,";

    let expected_rows = &mut ["1,-0.5,0,-0.5,true"];

    assert_cmd_succeeds_with_result(csv_content, expected_rows)
}

/// # Arguments
///
/// * `csv_content` - Input to the program
/// * `expected_rows` - Expected output rows, excluding header. Order is ignored.
fn assert_cmd_succeeds_with_result(
    csv_content: &str,
    expected_rows: &mut [&str],
) -> Result<(), Box<dyn std::error::Error>> {
    let csv_file = assert_fs::NamedTempFile::new("transactions.csv")?;
    csv_file.write_str(csv_content)?;

    let mut cmd = Command::cargo_bin(BIN_NAME)?;
    cmd.arg(csv_file.path());

    let assertion = cmd.assert().success();

    let output = std::str::from_utf8(&assertion.get_output().stdout)?;
    let mut rows = output.trim().split('\n');

    if let Some(header_row) = rows.next() {
        assert_eq!("client,available,held,total,locked", header_row);
    } else {
        assert!(false, "Missing header row");
    }

    // Remaining rows after header are the expected accounts
    let mut account_rows: Vec<&str> = rows.collect();
    account_rows.sort(); // Sort them because order is not important in the results

    // Also sort the expected results because order is not important in the results
    expected_rows.sort();

    assert_eq!(*expected_rows, *account_rows);

    Ok(())
}
