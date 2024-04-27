use std::io::{self, BufRead, Write};

use eyre::{Result, WrapErr};
use secrecy::SecretString;

pub fn prompt(message: &str) -> Result<String> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    stdout
        .write_all(message.as_bytes())
        .wrap_err("Failed to write to stdout while writing prompt")?;
    stdout
        .flush()
        .wrap_err("Failed to flush stdout while writing prompt")?;

    let stdin = io::stdin();
    let mut stdin = stdin.lock();

    let mut line = String::new();
    stdin
        .read_line(&mut line)
        .wrap_err("Failed to read line from stdin after a prompt")?;
    Ok(line)
}

pub fn prompt_secret(message: &str) -> Result<SecretString> {
    let secret = rpassword::prompt_password(message).wrap_err("Failed to read secret")?;

    Ok(secret.into())
}
