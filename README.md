# Payments Engine

## Overview

This command-line program implements a basic payments engine. It is assumed that the reader has access to the specification, which is not described here.

## Assumptions

The specification does not completely describe the intended behaviour of the program, so several assumptions have been made in lieu of clarification:

1. According to the spec, when a transaction is disputed, funds are to be moved from `available` to `held`. This means it really only makes sense for `deposit` type transactions to be disputable. Disputing a `withdrawal` doesn't make sense because the funds have already been withdrawn, which means there are no funds to hold. Therefore, `dispute` and related operations are only allowed against `deposit` transactions.[^1]
2. If a `deposit` is disputed when there is insufficient balance in `available` to withhold the funds for the disputed `deposit`, `available` is allowed to go negative. This makes sense if we assume that the entity managing the account funds is liable for funding any chargebacks, so that funding a chargeback does not depend on the client account having sufficient available funds. If `available` goes negative, this represents a deficit for the client: the client is in turn liable for that amount to the managing entity.
3. A transaction cannot have more than one open dispute at a time.
4. A transaction can be disputed more than once so long as any previous disputes have already been resolved.
5. When an account is locked, deposits/withdrawals are not allowed, but disputes can still be procesed.

[^1]: An example of a disputable deposit might be a client using a stolen credit card to deposit funds into their account. The dispute would presumably be raised by the credit card company to recover the funds. However, the spec says a dispute represents a _client's_ claim that a transaction was erroneous, so this assumption doesn't quite fit with that, but I can't think of any other way to reconcile the requirements.

## Performance considerations

Performance testing has _not_ been carried out, but the program has been designed to stream results from the transaction CSV rather than loading the entire thing into memory. This should allow large files to be ingested without blowing out memory usage.

If transactions were being streamed from many concurrent sources (e.g. TCP streams), one approach to improving throughput would be to use sharding to allow multiple threads to process results in parallel. For instance, we could create shard keys based on a hash of `client_id` and create a separate ledger for each shard key. Then, as results are streamed in, we could create a channel per shard which transactions would be pushed to, and then each channel could have a worker thread to handle updating the ledger.

## Testing methodology

The Rust type used to represent `Transaction` only allows for valid transactions: any invalid transaction rows will fail parsing. This means that we don't need to perform validation on transactions when ingesting into the ledger, and reduces the number of test cases we need to provide.

It is possible to test all edge cases via the command line, so I have opted to omit unit tests and provide a suite of tests that directly test the command line program in `tests/cli.rs`.

> **Note**: There are many edge cases that are NOT covered by these tests and there is plenty of scope for improvement and covering edge cases.

## Suggested improvements

- Support case-insensitive deserialization of transaction types
- Support multiple ongoing unresolved disputes against a single transaction (would require additional field in the transaction CSV to uniquely identify a dispute)
- Refactor the `Ledger` trait so that most of the core business logic is shared (doesn't live in `HashMapLedger`) and implementation-agnostic (i.e. can be re-used whether storage layer for the ledger is in-memory or otherwise)
- Use a more descriptive type than `String` for errors returned by `handle_transaction` on `Ledger`. This would allow us to swallow (ignore) some kinds of errors and continue, and panic on others.
- Expanded test coverage
- Performance testing for large result sets to ensure memory usage doesn't blow up