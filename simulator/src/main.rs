// Copyright 2025 Erst Users
// SPDX-License-Identifier: Apache-2.0

mod theme;
mod config;
mod cli;
mod ipc;
mod gas_optimizer;

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::json;
use soroban_env_host::events::Events;
use soroban_env_host::xdr::ReadXdr;
use std::collections::HashMap;
use std::io::{self, Read};
use std::panic;

use gas_optimizer::{BudgetMetrics, GasOptimizationAdvisor, OptimizationReport};

#[derive(Debug, Deserialize)]
struct SimulationRequest {
    envelope_xdr: String,
    result_meta_xdr: String,
    ledger_entries: Option<HashMap<String, String>>,
    timestamp: Option<i64>,
    ledger_sequence: Option<u32>,
    // Optional: Path to local WASM file for local replay
    wasm_path: Option<String>,
    // Optional: Mock arguments for local replay (JSON array of strings)
    mock_args: Option<Vec<String>>,
    profile: Option<bool>,
    #[serde(default)]
    enable_optimization_advisor: bool,
}

#[derive(Debug, Serialize, Clone)]
struct CategorizedEvent {
    event_type: String,
    contract_id: Option<String>,
    topics: Vec<String>,
    data: String,
}

#[derive(Debug, Serialize)]
struct SimulationResponse {
    status: String,
    error: Option<String>,
    events: Vec<String>,
    categorized_events: Vec<CategorizedEvent>,
    logs: Vec<String>,
    flamegraph: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    optimization_report: Option<OptimizationReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    budget_usage: Option<BudgetUsage>,
}

#[derive(Debug, Serialize)]
struct BudgetUsage {
    cpu_instructions: u64,
    memory_bytes: u64,
    operations_count: usize,
}

fn categorize_event_for_analyzer(
    event: &soroban_env_host::events::HostEvent,
) -> Result<String, String> {
    use soroban_env_host::xdr::{ContractEventBody, ContractEventType, ScVal};

    let contract_id = match &event.event.contract_id {
        Some(id) => format!("{:?}", id),
        None => "unknown".to_string(),
    };

    let event_type_str = match &event.event.type_ {
        ContractEventType::Contract => "contract",
        ContractEventType::System => "system",
        ContractEventType::Diagnostic => "diagnostic",
    };

    let (topics, _data_val) = match &event.event.body {
        ContractEventBody::V0(v0) => (&v0.topics, &v0.data),
    };

    let event_json = if let Some(first_topic) = topics.get(0) {
        let topic_str = format!("{:?}", first_topic);

        if topic_str.contains("require_auth") {
            let address = if let ScVal::Address(addr) = first_topic {
                format!("{:?}", addr)
            } else {
                "unknown".to_string()
            };

            json!({
                "type": "auth",
                "contract": contract_id,
                "address": address,
                "event_type": event_type_str,
            })
            .to_string()
        } else if topic_str.contains("set")
            || topic_str.contains("write")
            || topic_str.contains("storage")
        {
            json!({
                "type": "storage_write",
                "contract": contract_id,
                "event_type": event_type_str,
            })
            .to_string()
        } else if topic_str.contains("call") || topic_str.contains("invoke") {
            if let ScVal::Symbol(sym) = first_topic {
                json!({
                    "type": "contract_call",
                    "contract": contract_id,
                    "function": sym.to_string(),
                    "event_type": event_type_str,
                })
                .to_string()
            } else {
                json!({
                    "type": "contract_call",
                    "contract": contract_id,
                    "event_type": event_type_str,
                })
                .to_string()
            }
        } else {
            json!({
                "type": "other",
                "contract": contract_id,
                "event_type": event_type_str,
            })
            .to_string()
        }
    } else {
        json!({
            "type": "other",
            "contract": contract_id,
            "event_type": event_type_str,
        })
        .to_string()
    };

    Ok(event_json)
}

#[derive(Debug, Serialize, Deserialize)]
struct StructuredError {
    error_type: String,
    message: String,
    details: Option<String>,
}

fn main() {
    // Read JSON from Stdin
    let mut buffer = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut buffer) {
        let res = SimulationResponse {
            status: "error".to_string(),
            error: Some(format!("Failed to read stdin: {}", e)),
            events: vec![],
            categorized_events: vec![],
            logs: vec![],
            flamegraph: None,
            optimization_report: None,
            budget_usage: None,
        };
        println!("{}", serde_json::to_string(&res).unwrap());
        return;
    }

    // Parse Request
    let request: SimulationRequest = match serde_json::from_str(&buffer) {
        Ok(req) => req,
        Err(e) => {

        }
    };

    // Check if this is a local WASM replay (no network data)
    if let Some(wasm_path) = &request.wasm_path {
        return run_local_wasm_replay(wasm_path, &request.mock_args);
    }

    // Decode Envelope XDR
    let envelope = match base64::engine::general_purpose::STANDARD.decode(&request.envelope_xdr) {
        Ok(bytes) => match soroban_env_host::xdr::TransactionEnvelope::from_xdr(

        ) {
            Ok(env) => env,
            Err(e) => {
                return send_error(format!("Failed to parse Envelope XDR: {}", e));
            }
        },
        Err(e) => {
            return send_error(format!("Failed to decode Envelope Base64: {}", e));
        }
    };

    // Initialize Host
    let host = soroban_env_host::Host::default();
    host.set_diagnostic_level(soroban_env_host::DiagnosticLevel::Debug)
        .unwrap();

    // Override Ledger Info if provided
    if request.timestamp.is_some() || request.ledger_sequence.is_some() {
        host.with_mut_ledger_info(|ledger_info| {
            if let Some(ts) = request.timestamp {
                ledger_info.timestamp = ts as u64;
            }
            if let Some(seq) = request.ledger_sequence {
                ledger_info.sequence_number = seq;
            }
        })
        .unwrap();
    }
    // Populate Host Storage
    let mut loaded_entries_count = 0;
    if let Some(entries) = &request.ledger_entries {
        for (key_xdr, entry_xdr) in entries {

                    Ok(k) => k,
                    Err(e) => return send_error(format!("Failed to parse LedgerKey XDR: {}", e)),
                },
                Err(e) => return send_error(format!("Failed to decode LedgerKey Base64: {}", e)),
            };


                    Ok(e) => e,
                    Err(e) => return send_error(format!("Failed to parse LedgerEntry XDR: {}", e)),
                },
                Err(e) => return send_error(format!("Failed to decode LedgerEntry Base64: {}", e)),
            };

            loaded_entries_count += 1;
        }
    }


    let operations = match &envelope {
        soroban_env_host::xdr::TransactionEnvelope::Tx(tx_v1) => &tx_v1.tx.operations,
        soroban_env_host::xdr::TransactionEnvelope::TxV0(tx_v0) => &tx_v0.tx.operations,
        soroban_env_host::xdr::TransactionEnvelope::TxFeeBump(bump) => match &bump.tx.inner_tx {
            soroban_env_host::xdr::FeeBumpTransactionInnerTx::Tx(tx_v1) => &tx_v1.tx.operations,
        },
    };



            ];
            final_logs.extend(exec_logs);

            let response = SimulationResponse {
                status: "success".to_string(),
                error: None,
                events,
                categorized_events,
                logs: final_logs,
                flamegraph: flamegraph_svg,
                optimization_report,
                budget_usage: Some(budget_usage),
            };

            println!("{}", serde_json::to_string(&response).unwrap());
        }
        Ok(Err(host_error)) => {
            // Host error during execution (e.g., contract trap, validation failure)
            let structured_error = StructuredError {
                error_type: "HostError".to_string(),
                message: format!("{:?}", host_error),
                details: Some(format!(
                    "Contract execution failed with host error: {:?}",
                    host_error
                )),
            };

            let response = SimulationResponse {
                status: "error".to_string(),
                error: Some(serde_json::to_string(&structured_error).unwrap()),
                events: vec![],
                categorized_events: vec![],
                logs: vec![],
                flamegraph: None,
                optimization_report: None,
                budget_usage: None,
            };

            println!("{}", serde_json::to_string(&response).unwrap());
        }
        Err(panic_info) => {
            let panic_msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic".to_string()
            };

            let response = SimulationResponse {
                status: "error".to_string(),
                error: Some(format!("Simulator panicked: {}", panic_msg)),
                events: vec![],
                categorized_events: vec![],
                logs: vec![format!("PANIC: {}", panic_msg)],
                flamegraph: None,
                optimization_report: None,
                budget_usage: None,
            };
            println!("{}", serde_json::to_string(&response).unwrap());
        }
    }
}

fn execute_operations(
    _host: &soroban_env_host::Host,
    operations: &soroban_env_host::xdr::VecM<soroban_env_host::xdr::Operation, 100>,
) -> Result<Vec<String>, soroban_env_host::HostError> {
    let mut logs = vec![];

    for op in operations.iter() {
        if let soroban_env_host::xdr::OperationBody::InvokeHostFunction(host_fn_op) = &op.body {
            match &host_fn_op.host_function {
                soroban_env_host::xdr::HostFunction::InvokeContract(invoke_args) => {
                    logs.push("Found InvokeContract operation!".to_string());

                    let address = &invoke_args.contract_address;
                    let func_name = &invoke_args.function_name;
                    let invoke_args_vec = &invoke_args.args;

                    logs.push(format!("About to Invoke Contract: {:?}", address));
                    logs.push(format!("Function: {:?}", func_name));
                    logs.push(format!("Args Count: {}", invoke_args_vec.len()));
                }
                _ => {
                    logs.push("Skipping non-InvokeContract Host Function".to_string());
                }
            }
        }
    }
    Ok(logs)
}

fn categorize_events(events: &Events) -> Vec<CategorizedEvent> {
    use soroban_env_host::xdr::{ContractEventBody, ContractEventType, ScVal};

    events
        .0
        .iter()
        .filter_map(|event| {
            // Access body to get topics and data
            let (topics, data_val) = match &event.event.body {
                ContractEventBody::V0(v0) => (&v0.topics, &v0.data),
            };

            if !event.failed_call {
                let event_type = match &event.event.type_ {
                    ContractEventType::Contract => {
                        if let Some(topic) = topics.get(0) {
                            if let ScVal::Symbol(sym) = topic {
                                match sym.to_string().as_str() {
                                    s if s.contains("require_auth") => "require_auth",
                                    s if s.contains("set") || s.contains("write") => {
                                        "storage_write"
                                    }
                                    _ => "contract",
                                }
                            } else {
                                "contract"
                            }
                        } else {
                            "contract"
                        }
                    }
                    ContractEventType::System => "system",
                    ContractEventType::Diagnostic => {
                        if let Some(topic) = topics.get(0) {
                            if let ScVal::Symbol(sym) = topic {
                                match sym.to_string().as_str() {
                                    s if s.contains("fn_call") => "invocation",
                                    s if s.contains("fn_return") => "return",
                                    _ => "diagnostic",
                                }
                            } else {
                                "diagnostic"
                            }
                        } else {
                            "diagnostic"
                        }
                    }
                };

                Some(CategorizedEvent {
                    event_type: event_type.to_string(),
                    contract_id: event
                        .event
                        .contract_id
                        .as_ref()
                        .map(|id| format!("{:?}", id)),
                    topics: topics.iter().map(|t| format!("{:?}", t)).collect(),
                    data: format!("{:?}", data_val),
                })
            } else {
                None
            }
        })
        .collect()
}





        flamegraph: None,
        optimization_report: None,
        budget_usage: None,
    };
    println!("{}", serde_json::to_string(&res).unwrap());


