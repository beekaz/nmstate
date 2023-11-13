use log::warn;

use crate::{RouteEntry, Routes};

const SUPPORTED_ROUTE_SCOPE: [nispor::RouteScope; 2] =
    [nispor::RouteScope::Universe, nispor::RouteScope::Link];

const SUPPORTED_ROUTE_PROTOCOL: [nispor::RouteProtocol; 7] = [
    nispor::RouteProtocol::Boot,
    nispor::RouteProtocol::Static,
    nispor::RouteProtocol::Ra,
    nispor::RouteProtocol::Dhcp,
    nispor::RouteProtocol::Mrouted,
    nispor::RouteProtocol::KeepAlived,
    nispor::RouteProtocol::Babel,
];

const SUPPORTED_STATIC_ROUTE_PROTOCOL: [nispor::RouteProtocol; 2] =
    [nispor::RouteProtocol::Boot, nispor::RouteProtocol::Static];

const IPV4_DEFAULT_GATEWAY: &str = "0.0.0.0/0";
const IPV6_DEFAULT_GATEWAY: &str = "::/0";
const IPV4_EMPTY_NEXT_HOP_ADDRESS: &str = "0.0.0.0";
const IPV6_EMPTY_NEXT_HOP_ADDRESS: &str = "::";

pub(crate) fn get_routes(running_config_only: bool) -> Routes {
    let mut ret = Routes::new();
    let mut np_routes: Vec<nispor::Route> = Vec::new();

    let protocols = if running_config_only {
        SUPPORTED_STATIC_ROUTE_PROTOCOL.as_slice()
    } else {
        SUPPORTED_ROUTE_PROTOCOL.as_slice()
    };

    for protocol in protocols {
        let mut rt_filter = nispor::NetStateRouteFilter::default();
        rt_filter.protocol = Some(*protocol);
        let mut filter = nispor::NetStateFilter::minimum();
        filter.route = Some(rt_filter);
        match nispor::NetState::retrieve_with_filter(&filter) {
            Ok(np_state) => {
                for np_rt in np_state.routes {
                    np_routes.push(np_rt);
                }
            }
            Err(e) => {
                log::warn!(
                    "Failed to retrieve {:?} route via nispor: {}",
                    protocol,
                    e
                );
            }
        }
    }

    if !running_config_only {
        let mut running_routes = Vec::new();
        for np_route in np_routes
            .iter()
            .filter(|np_route| SUPPORTED_ROUTE_SCOPE.contains(&np_route.scope))
        {
            if is_multipath(np_route) {
                for route in flat_multipath_route(np_route) {
                    running_routes.push(route);
                }
            } else if np_route.oif.is_some() {
                running_routes.push(np_route_to_nmstate(np_route));
            }
        }
        ret.running = Some(running_routes);
    }

    let mut config_routes = Vec::new();
    for np_route in np_routes.iter().filter(|np_route| {
        SUPPORTED_ROUTE_SCOPE.contains(&np_route.scope)
            && SUPPORTED_STATIC_ROUTE_PROTOCOL.contains(&np_route.protocol)
    }) {
        if is_multipath(np_route) {
            for route in flat_multipath_route(np_route) {
                config_routes.push(route);
            }
        } else if np_route.oif.is_some() {
            config_routes.push(np_route_to_nmstate(np_route));
        }
    }
    ret.config = Some(config_routes);
    ret
}

fn np_route_to_nmstate(np_route: &nispor::Route) -> RouteEntry {
    let destination = match &np_route.dst {
        Some(dst) => Some(dst.to_string()),
        None => match np_route.address_family {
            nispor::AddressFamily::IPv4 => {
                Some(IPV4_DEFAULT_GATEWAY.to_string())
            }
            nispor::AddressFamily::IPv6 => {
                Some(IPV6_DEFAULT_GATEWAY.to_string())
            }
            _ => {
                warn!(
                    "Route {:?} is holding unknown IP family {:?}",
                    np_route, np_route.address_family
                );
                None
            }
        },
    };

    let next_hop_addr = if let Some(via) = &np_route.via {
        Some(via.to_string())
    } else if let Some(gateway) = &np_route.gateway {
        Some(gateway.to_string())
    } else {
        match np_route.address_family {
            nispor::AddressFamily::IPv4 => {
                Some(IPV4_EMPTY_NEXT_HOP_ADDRESS.to_string())
            }
            nispor::AddressFamily::IPv6 => {
                Some(IPV6_EMPTY_NEXT_HOP_ADDRESS.to_string())
            }
            _ => {
                warn!(
                    "Route {:?} is holding unknown IP family {:?}",
                    np_route, np_route.address_family
                );
                None
            }
        }
    };

    let mut route_entry = RouteEntry::new();
    route_entry.destination = destination;
    route_entry.next_hop_iface = np_route.oif.as_ref().cloned();
    route_entry.next_hop_addr = next_hop_addr;
    route_entry.metric = np_route.metric.map(i64::from);
    route_entry.table_id = Some(np_route.table);

    route_entry
}

fn is_multipath(np_route: &nispor::Route) -> bool {
    np_route
        .multipath
        .as_ref()
        .map(|m| !m.is_empty())
        .unwrap_or_default()
}

fn flat_multipath_route(np_route: &nispor::Route) -> Vec<RouteEntry> {
    let mut ret: Vec<RouteEntry> = Vec::new();
    if let Some(mpath_routes) = np_route.multipath.as_ref() {
        for mp_route in mpath_routes {
            let mut new_np_route = np_route.clone();
            new_np_route.via = Some(mp_route.via.to_string());
            new_np_route.oif = Some(mp_route.iface.to_string());
            let mut route = np_route_to_nmstate(&new_np_route);
            if np_route.address_family == nispor::AddressFamily::IPv4 {
                route.weight = Some(mp_route.weight);
            }
            ret.push(route);
        }
    }
    ret
}
