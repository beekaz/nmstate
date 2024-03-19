// SPDX-License-Identifier: Apache-2.0

use crate::HostNameState;
use crate::Interfaces;
use crate::NetworkState;
use crate::OvnConfiguration;
use crate::RouteRules;
use crate::Routes;

#[test]
fn test_invalid_top_key() {
    let result = serde_yaml::from_str::<NetworkState>(
        r"---
invalid_key: abc
",
    );

    assert!(result.is_err());
}

#[test]
fn test_invalid_top_type() {
    let result = serde_yaml::from_str::<NetworkState>(
        r"---
- invalid_key: abc
",
    );

    assert!(result.is_err());
}

#[test]
fn test_network_state_debug() {
    let mut network_state = NetworkState {
        hostname: Some(HostNameState::default()),
        dns: None,
        rules: RouteRules::default(),
        routes: Routes::default(),
        interfaces: Interfaces::default(),
        ovsdb: None,
        timeout: Some(60),
        no_verify: true,
        no_commit: false,
        ovn: OvnConfiguration::default(),
        include_secrets: true,
        include_status_data: false,
        running_config_only: true,
        memory_only: false,
        kernel_only: false,
    };

    network_state.hide_secrets();

    let debug_output = format!("{:?}", network_state);

    // Assert that the debug output contains the expected fields
    assert!(debug_output.contains("hostname"));
    assert!(debug_output.contains("dns"));
    assert!(debug_output.contains("rules"));
    assert!(debug_output.contains("routes"));
    assert!(debug_output.contains("interfaces"));
    assert!(debug_output.contains("ovsdb"));
    assert!(debug_output.contains("timeout"));
    assert!(debug_output.contains("no_verify"));
    assert!(debug_output.contains("no_commit"));
    assert!(debug_output.contains("include_secrets"));
    assert!(debug_output.contains("include_status_data"));
    assert!(debug_output.contains("running_config_only"));
    assert!(debug_output.contains("memory_only"));

    // Assert that the debug output contains the hidden password
    assert!(debug_output.contains("<_password_hid_by_nmstate>"));
}
