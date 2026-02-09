use candid::{CandidType, Principal};
use hex::decode;
use ic_management_canister_types::CanisterSettings;
use pocket_ic::common::rest::IcpConfig;
use pocket_ic::common::rest::IcpConfigFlag;
use pocket_ic::common::rest::RawEffectivePrincipal;
use pocket_ic::{PocketIc, PocketIcBuilder, RejectResponse, call_candid_as};
use serde::Serialize;
use std::path::PathBuf;
use std::str::Chars;
use std::time::Duration;

// Canister Management Commands:
// 1. install <canister_id> <wasm_path> <init_args>
//    - Creates and installs a new canister with the given WASM
//    - init_args can be empty ("") or DIDL-encoded
//
// 2. reinstall <canister_id> <wasm_path> <init_args>
//    - Reinstalls an existing canister with new WASM
//    - Completely resets the canister state
//    - init_args can be empty ("") or DIDL-encoded
//
// 3. upgrade <canister_id> <wasm_path> <upgrade_args>
//    - Upgrades an existing canister with new WASM
//    - Preserves canister state
//    - upgrade_args can be empty ("") or DIDL-encoded
//
// Canister Interaction Commands:
// 1. ingress <canister_id> <method_name> <args>
//    - Makes an ingress call to a canister method
//    - args can be empty ("") or DIDL-encoded
//    - Returns the response or error
//    - Special case: __motoko_stabilize_before_upgrade is a special ingress call
//      used for stabilization before upgrade
//
// 2. query <canister_id> <method_name> <args>
//    - Makes a query call to a canister method
//    - args can be empty ("") or DIDL-encoded
//    - Returns the response or error

#[derive(Debug, Clone, Copy)]
pub enum SubnetType {
    Application,
    System,
}

#[derive(Debug, Clone)]
pub enum TestCommand {
    Install {
        canister_id: String,
        wasm_path: PathBuf,
        init_args: String,
    },
    Reinstall {
        canister_id: String,
        wasm_path: PathBuf,
        init_args: String,
    },
    Upgrade {
        canister_id: String,
        wasm_path: PathBuf,
        upgrade_args: String,
    },
    Ingress {
        canister_id: String,
        method_name: String,
        args: String,
    },
    Query {
        canister_id: String,
        method_name: String,
        args: String,
    },
}

#[derive(CandidType, Serialize, Debug, PartialEq)]
pub enum WasmMemoryPersistence {
    /// Preserve heap memory.
    #[serde(rename = "keep")]
    Keep,
    /// Clear heap memory.
    #[serde(rename = "replace")]
    _Replace,
}
#[derive(CandidType, Serialize, Debug, PartialEq)]
pub struct UpgradeFlags {
    pub skip_pre_upgrade: Option<bool>,
    pub wasm_memory_persistence: Option<WasmMemoryPersistence>,
}

#[derive(CandidType, Serialize, Debug, PartialEq)]
pub enum CanisterInstallModeV2 {
    #[serde(rename = "install")]
    Install,
    #[serde(rename = "reinstall")]
    _Reinstall,
    #[serde(rename = "upgrade")]
    Upgrade(Option<UpgradeFlags>),
}

#[derive(CandidType, Serialize, Debug, PartialEq)]
pub struct InstallCodeArgument {
    pub mode: CanisterInstallModeV2,
    pub canister_id: Principal,
    pub wasm_module: Vec<u8>,
    pub arg: Vec<u8>,
}

#[derive(Debug)]
enum Radix {
    Bin = 2,
    Hex = 16,
}

fn parse_escape(chars: &mut Chars<'_>, radix: Radix) -> Result<u8, String> {
    let len = match radix {
        Radix::Bin => 8,
        Radix::Hex => 2,
    };
    let s = chars.take(len).collect::<String>();
    if s.len() >= len {
        u8::from_str_radix(&s, radix as u32).map_err(|e| e.to_string())
    } else {
        Err(format!(
            "Escape sequence for radix {:?} too short: {}",
            radix, s
        ))
    }
}

fn parse_quoted(quoted_str: &str) -> Result<Vec<u8>, String> {
    if !quoted_str.is_ascii() {
        return Err("Only ASCII strings are allowed.".to_string());
    }

    let mut chars = quoted_str.chars();
    let mut res: Vec<u8> = Vec::new();
    let mut escaped = false;

    if Some('"') != chars.next() {
        return Err("Double-quoted string must be enclosed in double quotes.".to_string());
    }

    let mut c = chars.next();
    while let Some(cur) = c {
        if escaped {
            let b = match cur {
                'x' => parse_escape(&mut chars, Radix::Hex)?,
                'b' => parse_escape(&mut chars, Radix::Bin)?,
                '"' => b'"',
                '\\' => b'\\',
                _ => return Err(format!("Illegal escape sequence {}", cur)),
            };
            res.push(b);
            escaped = false;
        } else {
            match cur {
                '\\' => escaped = true,
                '"' => {
                    chars.next(); // consume '"'
                    break;
                }
                _ => res.push(cur as u8),
            }
        }
        c = chars.next();
    }

    if chars.next().is_some() {
        return Err("Trailing characters after string terminator.".to_string());
    }

    Ok(res)
}

fn parse_hex(s: &str) -> Result<Vec<u8>, String> {
    if let Some(s) = s.strip_prefix("0x") {
        decode(s).map_err(|e| e.to_string())
    } else {
        Err(format!("Illegal hex character sequence {}.", s))
    }
}

fn parse_str_args(input_str: &str) -> Result<Vec<u8>, String> {
    if input_str.starts_with('"') {
        parse_quoted(input_str)
    } else {
        parse_hex(input_str)
    }
}

fn contains_icp_private_custom_section(wasm_binary: &[u8], name: &str) -> Result<bool, String> {
    use wasmparser::{Parser, Payload::CustomSection};

    let icp_section_name = format!("icp:private {}", name);
    let parser = Parser::new(0);
    for payload in parser.parse_all(wasm_binary) {
        if let CustomSection(reader) = payload.map_err(|e| format!("Wasm parsing error: {}", e))?
            && reader.name() == icp_section_name
        {
            return Ok(true);
        }
    }
    Ok(false)
}

trait ResultExtractor {
    fn extract(&self, command: TestCommand) -> String;
}

// For ingress messages (ingress, reinstall, upgrade, install), the output prepends always "ingress Completed: Reply"
// For query messages, the output prepends always "Ok: Reply".
impl ResultExtractor for () {
    fn extract(&self, command: TestCommand) -> String {
        match command {
            TestCommand::Ingress { .. }
            | TestCommand::Reinstall { .. }
            | TestCommand::Upgrade { .. }
            | TestCommand::Install { .. } => "ingress Completed: Reply: 0x4449444c0000".to_string(),
            TestCommand::Query { .. } => "Ok: Reply: 0x4449444c0000".to_string(),
        }
    }
}

// For ingress messages (ingress, reinstall, upgrade, install), the output prepends always "ingress Completed: Reply"
// For query messages, the output prepends always "Ok: Reply".
impl ResultExtractor for Vec<u8> {
    fn extract(&self, command: TestCommand) -> String {
        let hex_str: String = self.iter().map(|b| format!("{:02x}", b)).collect();
        match command {
            TestCommand::Ingress { .. }
            | TestCommand::Reinstall { .. }
            | TestCommand::Upgrade { .. }
            | TestCommand::Install { .. } => {
                format!("ingress Completed: Reply: 0x{}", hex_str)
            }
            TestCommand::Query { .. } => format!("Ok: Reply: 0x{}", hex_str),
        }
    }
}

impl TestCommand {
    fn principal_from_text(text: &str) -> Result<Principal, std::io::Error> {
        Principal::from_text(text)
            .map_err(|e| std::io::Error::other(format!("Failed to parse canister id: {}", e)))
    }

    pub fn read_wasm_file(wasm_path: &PathBuf) -> Result<Vec<u8>, std::io::Error> {
        std::fs::read(wasm_path)
            .map_err(|e| std::io::Error::other(format!("Failed to read wasm file: {}", e)))
    }

    fn parse_args(input_str: &str) -> Result<Vec<u8>, std::io::Error> {
        parse_str_args(input_str)
            .map_err(|e| std::io::Error::other(format!("Failed to parse args: {}", e)))
    }

    fn create_canister(&self, server: &mut PocketIc, canister_id: &str) -> std::io::Result<()> {
        let canister_principal = Self::principal_from_text(canister_id)?;
        let sender = Principal::anonymous();
        let result = server.create_canister_with_id(
            Some(sender),
            Some(CanisterSettings {
                controllers: Some(vec![sender, canister_principal]),
                ..Default::default()
            }),
            canister_principal,
        );
        server.add_cycles(canister_principal, 1_000_000_000_000_000);

        if let Err(e) = result {
            return Err(std::io::Error::other(format!(
                "Failed to create canister: {:?}",
                e
            )));
        } else {
            println!(
                "ingress Completed: Reply: 0x4449444c016c01b3c4b1f204680100010a00000000000000000101"
            );
        }
        Ok(())
    }

    fn install_command(
        &self,
        server: &mut PocketIc,
        canister_id: &str,
        wasm_path: &PathBuf,
        init_args: &str,
    ) -> std::io::Result<()> {
        let wasm_bytes = Self::read_wasm_file(wasm_path)?;
        let args = Self::parse_args(init_args)?;
        let canister_principal = Self::principal_from_text(canister_id)?;
        // server.install_canister(canister_principal, wasm_bytes, args, None);
        // Use the call_candid_as method instead of install_canister because install_canister unwraps() and
        // for some of our tests this leads to a panic.
        // Therefore we need to catch the error and print it.
        let res: Result<(), _> = call_candid_as(
            server,
            Principal::management_canister(),
            RawEffectivePrincipal::CanisterId(canister_principal.as_slice().to_vec()),
            Principal::anonymous(),
            "install_code",
            (InstallCodeArgument {
                mode: CanisterInstallModeV2::Install,
                canister_id: canister_principal,
                wasm_module: wasm_bytes,
                arg: args,
            },),
        );
        println!("{}", self.handle_result_and_get_response(res));
        Ok(())
    }

    // Checks if the result is an error or a proper response.
    // A response can be of type () or Vev<u8>.
    // The error is always a RejectResponse.
    fn handle_result_and_get_response<T: ResultExtractor>(
        &self,
        res: Result<T, RejectResponse>,
    ) -> String {
        match res {
            Ok(t) => t.extract(self.clone()),
            Err(e) => {
                // For ingress messages (ingress, reinstall, upgrade, install), the output prepends always "ingress Completed: Reply"
                // For query messages, the output prepends always "Ok: Reply".
                match &self {
                    TestCommand::Ingress { .. }
                    | TestCommand::Reinstall { .. }
                    | TestCommand::Upgrade { .. }
                    | TestCommand::Install { .. } => {
                        format!("ingress Err: {}: {}", e.error_code, e.reject_message)
                    }
                    TestCommand::Query { .. } => {
                        format!("Err: {}: {}", e.error_code, e.reject_message)
                    }
                }
            }
        }
    }

    fn reinstall_command(
        &self,
        server: &mut PocketIc,
        canister_id: &str,
        wasm_path: &PathBuf,
        init_args: &str,
    ) -> std::io::Result<()> {
        let canister_principal = Self::principal_from_text(canister_id)?;
        let wasm_bytes = Self::read_wasm_file(wasm_path)?;
        let args = Self::parse_args(init_args)?;
        let res = server.reinstall_canister(canister_principal, wasm_bytes, args, None);
        println!("{}", self.handle_result_and_get_response(res));
        Ok(())
    }

    fn upgrade_command(
        &self,
        server: &mut PocketIc,
        canister_id: &str,
        wasm_path: &PathBuf,
        upgrade_args: &str,
    ) -> std::io::Result<()> {
        let canister_principal = Self::principal_from_text(canister_id)?;
        let wasm_bytes = Self::read_wasm_file(wasm_path)?;
        let args = Self::parse_args(upgrade_args)?;

        // Use the call_candid_as method instead of upgrade_canister.
        // This is important because it will allow us to specify whether the wasm memory
        // can be kept or not.
        let wasm_memory_persistence = if contains_icp_private_custom_section(
            wasm_bytes.as_ref(),
            "enhanced-orthogonal-persistence",
        )
        .unwrap_or(false)
        {
            Some(UpgradeFlags {
                skip_pre_upgrade: Some(false),
                wasm_memory_persistence: Some(WasmMemoryPersistence::Keep),
            })
        } else {
            Some(UpgradeFlags {
                skip_pre_upgrade: Some(false),
                wasm_memory_persistence: None,
            })
        };
        let arg = InstallCodeArgument {
            mode: CanisterInstallModeV2::Upgrade(wasm_memory_persistence),
            canister_id: canister_principal,
            wasm_module: wasm_bytes,
            arg: args,
        };
        let res: Result<(), _> = call_candid_as(
            server,
            Principal::management_canister(),
            RawEffectivePrincipal::CanisterId(canister_principal.as_slice().to_vec()),
            Principal::anonymous(),
            "install_code",
            (arg,),
        );
        println!("{}", self.handle_result_and_get_response(res));
        Ok(())
    }

    fn ingress_command(
        &self,
        server: &mut PocketIc,
        canister_id: &str,
        method_name: &str,
        args: &str,
    ) -> std::io::Result<()> {
        let canister_principal = Self::principal_from_text(canister_id)?;
        let payload = Self::parse_args(args)?;

        let res = match method_name {
            "__motoko_stabilize_before_upgrade" => server.update_call(
                canister_principal,
                Principal::anonymous(),
                method_name,
                payload,
            ),
            "stop_canister" => {
                let principal_bytes = &payload[payload.len() - 10..];
                let principal = Principal::from_slice(principal_bytes);
                let _ = server.stop_canister(principal, None);
                Ok(vec![0x44, 0x49, 0x44, 0x4c, 0x00, 0x00])
            }
            "start_canister" => {
                let principal_bytes = &payload[payload.len() - 10..];
                let principal = Principal::from_slice(principal_bytes);
                let _ = server.start_canister(principal, None);
                Ok(vec![0x44, 0x49, 0x44, 0x4c, 0x00, 0x00])
            }
            _ => {
                // Certain tests are of form await (with_timeout = X) and we need to wait
                // for the call either to be completed or to timeout.
                // We do this by submitting the call and then polling its status.
                let res = server
                    .submit_call(
                        canister_principal,
                        Principal::anonymous(),
                        method_name,
                        payload,
                    )
                    .map_err(|e| {
                        std::io::Error::other(format!("Failed to submit call: {:?}", e))
                    })?;
                while server.ingress_status(res.clone()).is_none() {
                    server.tick();
                    server.advance_time(Duration::from_secs(1));
                }
                // Safe to unwrap because we know that the status is not none.
                server.ingress_status(res.clone()).unwrap()
            }
        };

        println!("{}", self.handle_result_and_get_response(res));
        Ok(())
    }

    fn query_command(
        &self,
        server: &mut PocketIc,
        canister_id: &str,
        method_name: &str,
        args: &str,
    ) -> std::io::Result<()> {
        let canister_principal = Self::principal_from_text(canister_id)?;
        let payload = Self::parse_args(args)?;

        let res = server.query_call(
            canister_principal,
            Principal::anonymous(),
            method_name,
            payload,
        );
        println!("{}", self.handle_result_and_get_response(res));
        Ok(())
    }

    pub fn execute(&self, server: &mut PocketIc) -> std::io::Result<()> {
        match self {
            TestCommand::Install {
                canister_id,
                wasm_path,
                init_args,
            } => self.install_command(server, canister_id, wasm_path, init_args),
            TestCommand::Reinstall {
                canister_id,
                wasm_path,
                init_args,
            } => self.reinstall_command(server, canister_id, wasm_path, init_args),
            TestCommand::Upgrade {
                canister_id,
                wasm_path,
                upgrade_args,
            } => self.upgrade_command(server, canister_id, wasm_path, upgrade_args),
            TestCommand::Ingress {
                canister_id,
                method_name,
                args,
            } => self.ingress_command(server, canister_id, method_name, args),
            TestCommand::Query {
                canister_id,
                method_name,
                args,
            } => self.query_command(server, canister_id, method_name, args),
        }?;
        Ok(())
    }
}

pub fn parse_commands(content: &str) -> std::io::Result<Vec<TestCommand>> {
    let mut commands = Vec::new();

    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse commands
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        let command = match parts[0] {
            // if we encounter a create, just ignore it.
            "create" => continue,
            "install" => {
                if parts.len() != 4 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "install command requires 3 arguments",
                    ));
                }
                TestCommand::Install {
                    canister_id: parts[1].to_string(),
                    wasm_path: PathBuf::from(parts[2]),
                    init_args: parts[3].to_string(),
                }
            }
            "reinstall" => {
                if parts.len() != 4 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "reinstall command requires 3 arguments",
                    ));
                }
                TestCommand::Reinstall {
                    canister_id: parts[1].to_string(),
                    wasm_path: PathBuf::from(parts[2]),
                    init_args: parts[3].to_string(),
                }
            }
            "upgrade" => {
                if parts.len() != 4 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "upgrade command requires 3 arguments",
                    ));
                }
                TestCommand::Upgrade {
                    canister_id: parts[1].to_string(),
                    wasm_path: PathBuf::from(parts[2]),
                    upgrade_args: parts[3].to_string(),
                }
            }
            "ingress" => {
                if parts.len() != 4 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "ingress command requires 3 arguments",
                    ));
                }
                TestCommand::Ingress {
                    canister_id: parts[1].to_string(),
                    method_name: parts[2].to_string(),
                    args: parts[3].to_string(),
                }
            }
            "query" => {
                if parts.len() != 4 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "query command requires 3 arguments",
                    ));
                }
                TestCommand::Query {
                    canister_id: parts[1].to_string(),
                    method_name: parts[2].to_string(),
                    args: parts[3].to_string(),
                }
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Unknown command: {}", parts[0]),
                ));
            }
        };

        commands.push(command);
    }

    Ok(commands)
}

pub struct TestRunner {
    commands: Vec<TestCommand>,
    subnet_type: SubnetType,
}

impl TestRunner {
    pub fn new(commands: Vec<TestCommand>, subnet_type: SubnetType) -> Self {
        Self {
            commands,
            subnet_type,
        }
    }

    pub fn run(&self) -> std::io::Result<()> {
        let ic_config = IcpConfig {
            canister_backtrace: Some(IcpConfigFlag::Disabled),
            beta_features: Some(IcpConfigFlag::Enabled),
            ..Default::default()
        };
        let mut server = match self.subnet_type {
            SubnetType::Application => PocketIcBuilder::new().with_application_subnet(),
            SubnetType::System => PocketIcBuilder::new()
                .with_system_subnet()
                .with_nns_subnet(),
        }
        .with_icp_config(ic_config)
        .build();

        // Get the first canister id found in the commands.
        let canister_id = self.commands.iter().find_map(|command| match command {
            TestCommand::Install { canister_id, .. }
            | TestCommand::Reinstall { canister_id, .. }
            | TestCommand::Upgrade { canister_id, .. } => Some(canister_id),
            _ => None,
        });
        if let Some(canister_id) = canister_id {
            self.commands[0].create_canister(&mut server, canister_id)?;
        }
        for command in &self.commands {
            // command: {:?}", command);
            command.execute(&mut server)?;
        }
        Ok(())
    }
}

pub fn run_cmdline_test(buffer: String, subnet_type: SubnetType) {
    match parse_commands(&buffer) {
        Ok(commands) => {
            let test_runner = TestRunner::new(commands, subnet_type);
            let _ = test_runner.run();
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
