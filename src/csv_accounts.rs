use crate::ledger::Account;

pub(crate) struct Writer<W>(W);

impl<W> Writer<W> {
    pub fn from_writer(writer: W) -> Writer<W> {
        Writer(writer)
    }
}

impl<W: std::io::Write> Writer<W> {
    pub fn write_all<'a, I: Iterator<Item = &'a Account>>(
        self,
        accounts_iterator: I,
    ) -> Result<(), csv::Error> {
        let mut writer = csv::Writer::from_writer(self.0);

        writer.write_record(&["client", "available", "held", "total", "locked"])?;

        for account in accounts_iterator {
            match account {
                Account {
                    client_id,
                    available,
                    held,
                    is_locked,
                } => {
                    let fields = [
                        client_id.to_string(),
                        available.to_string(),
                        held.to_string(),
                        account.total().to_string(),
                        is_locked.to_string(),
                    ];
                    writer.write_record(fields)?;
                }
            };
        }

        Ok(())
    }
}
