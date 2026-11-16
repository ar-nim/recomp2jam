//! Mint the `OTwo.exe` launch argv (one `AuthParameters` blob plus five
//! gateway mirrors) from the identity fields a Memoryer session carries.
//!
//! The rig needs this for the exe leg: the 7.55 client reaches a rendered
//! scene only with the pinned `OTwo.exe <blob> (|test|??|<addr>|<port>) x5`
//! shape (`docs/wine-boot.md`), and a hand-built argv is what the ticket
//! brief replaced with this builder (`o2jam_net::auth_parameters`, #265).
//!
//! Usage:
//!   cargo run -p o2jam-net --example mint_launch_argv -- \
//!       --user-index-id 1 --username <session-guid> --level 7 \
//!       --gender 1 --nickname tester --rank 0 \
//!       --gateway-address 127.0.0.1 --gateway-port 15010
//!
//! Prints one argument per line, in argv order: the blob first, then five
//! identical mirrors. Newlines cannot appear in a blob (hex only) nor in a
//! mirror token, so one word per line is unambiguous.

use std::process::ExitCode;

use o2jam_net::auth_parameters::{assemble_launch_arguments, AuthParameters};

/// Identity fields the launcher blob carries from a Memoryer session.
struct SessionIdentity {
    user_index_id: String,
    username: String,
    level: String,
    gender: String,
    nickname: String,
    rank: String,
    gateway_address: String,
    gateway_port: String,
}

impl SessionIdentity {
    fn into_parameters(self) -> AuthParameters {
        AuthParameters {
            user_index_id: self.user_index_id,
            username: self.username,
            level: self.level,
            gender: self.gender,
            nickname: self.nickname,
            rank: self.rank,
            gateway_address: self.gateway_address,
            gateway_port: self.gateway_port,
            ..AuthParameters::default()
        }
    }
}

const USAGE: &str = "usage: mint_launch_argv --user-index-id N --username TOKEN \
--level N --gender N --nickname NAME --rank N \
--gateway-address ADDR --gateway-port PORT";

fn parse_flags(arguments: Vec<String>) -> Result<SessionIdentity, String> {
    let mut user_index_id = String::new();
    let mut username = String::new();
    let mut level = String::new();
    let mut gender = String::new();
    let mut nickname = String::new();
    let mut rank = String::new();
    let mut gateway_address = "127.0.0.1".to_owned();
    let mut gateway_port = "15010".to_owned();

    let mut pending = arguments.into_iter();
    while let Some(flag) = pending.next() {
        let value = pending
            .next()
            .ok_or_else(|| format!("{flag} needs a value"))?;
        match flag.as_str() {
            "--user-index-id" => user_index_id = value,
            "--username" => username = value,
            "--level" => level = value,
            "--gender" => gender = value,
            "--nickname" => nickname = value,
            "--rank" => rank = value,
            "--gateway-address" => gateway_address = value,
            "--gateway-port" => gateway_port = value,
            other => return Err(format!("unknown flag {other}")),
        }
    }

    if user_index_id.is_empty() || username.is_empty() {
        return Err("--user-index-id and --username are required".to_owned());
    }
    Ok(SessionIdentity {
        user_index_id,
        username,
        level,
        gender,
        nickname,
        rank,
        gateway_address,
        gateway_port,
    })
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let identity = match parse_flags(arguments) {
        Ok(identity) => identity,
        Err(message) => {
            eprintln!("mint_launch_argv: {message}");
            eprintln!("{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let parameters = identity.into_parameters();
    let argv = assemble_launch_arguments(
        &parameters,
        &parameters.gateway_address,
        &parameters.gateway_port,
    );
    for word in argv {
        println!("{word}");
    }
    ExitCode::SUCCESS
}
