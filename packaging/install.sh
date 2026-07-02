#!/usr/bin/env bash
set -euo pipefail

prefix="${PREFIX:-/usr/local}"
config_dir="${CONFIG_DIR:-/etc/biz-tunnel}"
systemd_dir="${SYSTEMD_DIR:-/etc/systemd/system}"
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
asset_dir="${ASSET_DIR:-${script_dir}/assets}"
cert_source_dir="${CERT_SOURCE_DIR:-${asset_dir}/certs}"
binary="${BINARY:-${asset_dir}/biz-tunnel}"
relay_config_source="${RELAY_CONFIG:-${asset_dir}/relay.toml}"
agent_config_source="${AGENT_CONFIG:-${asset_dir}/agent.toml}"
role="${ROLE:-}"
overwrite_config="${OVERWRITE_CONFIG:-0}"

if [[ "${EUID}" -ne 0 ]]; then
  echo "install.sh must run as root" >&2
  exit 1
fi

if ! getent group biz-tunnel >/dev/null; then
  groupadd --system biz-tunnel
fi

if ! id -u biz-tunnel >/dev/null 2>&1; then
  useradd --system --gid biz-tunnel --home-dir /nonexistent --shell /usr/bin/nologin biz-tunnel
fi

if [[ -z "${role}" ]]; then
  if [[ ! -t 0 ]]; then
    echo "set ROLE=relay or ROLE=agent when running non-interactively" >&2
    exit 1
  fi
  while [[ "${role}" != "relay" && "${role}" != "agent" ]]; do
    read -r -p "Select node role [relay/agent]: " role
  done
fi

case "${role}" in
  relay)
    config_path="${config_dir}/relay.toml"
    admin_url="http://127.0.0.1:18080"
    unit_requires=""
    unit_wants="network-online.target"
    unit_after="network-online.target"
    ;;
  agent)
    config_path="${config_dir}/agent.toml"
    admin_url="http://127.0.0.1:18081"
    unit_requires="Requires=sdp-headless.service"
    unit_wants="network-online.target sdp-headless.service"
    unit_after="network-online.target sdp-headless.service"
    ;;
  *)
    echo "ROLE must be relay or agent" >&2
    exit 1
    ;;
esac

if [[ ! -x "${binary}" ]]; then
  echo "missing executable binary: ${binary}" >&2
  exit 1
fi

if [[ ! -f "${relay_config_source}" ]]; then
  echo "missing embedded relay config: ${relay_config_source}" >&2
  exit 1
fi

if [[ ! -f "${agent_config_source}" ]]; then
  echo "missing embedded agent config: ${agent_config_source}" >&2
  exit 1
fi

install -d -m 0755 "${prefix}/bin"
install -d -m 0750 -o biz-tunnel -g biz-tunnel "${config_dir}"
install -d -m 0755 "${systemd_dir}"

install -m 0755 "${binary}" "${prefix}/bin/biz-tunnel"

if [[ -d "${cert_source_dir}" ]]; then
  install -d -m 0750 -o biz-tunnel -g biz-tunnel "${config_dir}/certs"
  install -m 0644 -o biz-tunnel -g biz-tunnel "${cert_source_dir}/ca.pem" "${config_dir}/certs/ca.pem"
  install -m 0644 -o biz-tunnel -g biz-tunnel "${cert_source_dir}/server.pem" "${config_dir}/certs/server.pem"
  install -m 0640 -o biz-tunnel -g biz-tunnel "${cert_source_dir}/server.key" "${config_dir}/certs/server.key"
fi

if [[ ! -f "${config_dir}/relay.toml" || "${overwrite_config}" == "1" ]]; then
  install -m 0640 -o biz-tunnel -g biz-tunnel "${relay_config_source}" "${config_dir}/relay.toml"
else
  echo "keep existing config: ${config_dir}/relay.toml"
fi

if [[ ! -f "${config_dir}/agent.toml" || "${overwrite_config}" == "1" ]]; then
  install -m 0640 -o biz-tunnel -g biz-tunnel "${agent_config_source}" "${config_dir}/agent.toml"
else
  echo "keep existing config: ${config_dir}/agent.toml"
fi

cat >"${systemd_dir}/biz-tunnel.service" <<EOF
[Unit]
Description=Biz Tunnel
Wants=${unit_wants}
After=${unit_after}
${unit_requires}

[Service]
User=biz-tunnel
Group=biz-tunnel
EnvironmentFile=${config_dir}/service.env
ExecStart=${prefix}/bin/biz-tunnel --config \${BIZ_TUNNEL_CONFIG}
ExecReload=${prefix}/bin/biz-tunnel reload --admin \${BIZ_TUNNEL_ADMIN}
Restart=on-failure
RestartSec=3
LimitNOFILE=1048576
AmbientCapabilities=CAP_NET_BIND_SERVICE
CapabilityBoundingSet=CAP_NET_BIND_SERVICE
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=${config_dir}

[Install]
WantedBy=multi-user.target
EOF

cat >"${config_dir}/service.env" <<EOF
BIZ_TUNNEL_ROLE=${role}
BIZ_TUNNEL_CONFIG=${config_path}
BIZ_TUNNEL_ADMIN=${admin_url}
EOF
chown biz-tunnel:biz-tunnel "${config_dir}/service.env"
chmod 0640 "${config_dir}/service.env"

systemctl daemon-reload

cat <<EOF
Installed biz-tunnel.

Next steps:
  1. Review ${config_path}
  2. Run: systemctl enable --now biz-tunnel.service
EOF
