# Call records

voipctl reads call records through the local MariaDB client. Asterisk remains responsible for writing them through an existing ODBC connection.

For a new database, review [the optional schema](../sql/cdr.sql), then create the table:

```sh
mariadb asteriskcdr < sql/cdr.sql
```

The database must already exist. The script fails if `cdr` exists; it does not migrate or replace an existing table. Back up existing data and adapt your schema separately. No database changes run during installation or `apply`.

Configure `cdr_adaptive_odbc` to use your existing connection and table `cdr`, with `alias start => calldate`. The generated dialplan supplies `accountcode`, `customer_ext`, `dialed`, `src_ip`, and `userfield`. Use the [official configuration reference](https://github.com/asterisk/asterisk/blob/22/configs/samples/cdr_adaptive_odbc.conf.sample) for connection mapping.

```sh
sudo voipctl cdr --today --summary
sudo voipctl cdr --customer main --limit 100
sudo voipctl server --cdr-database asteriskcdr
```

Use separate database identities for the Asterisk writer and the read-only CLI where possible. `--today` follows the database session's date; align its timezone with Asterisk's CDR timestamps. Retention and backups remain operator responsibilities. The schema has not been exercised against a live database.
