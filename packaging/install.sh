#!/usr/bin/env bash
set -euo pipefail

prefix="${PREFIX:-/usr/local}"
config_dir="${CONFIG_DIR:-/etc/biz-tunnel}"
systemd_dir="${SYSTEMD_DIR:-/etc/systemd/system}"
build_dir="${BUILD_DIR:-target/release}"

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

install -d -m 0755 "${prefix}/bin"
install -d -m 0750 -o biz-tunnel -g biz-tunnel "${config_dir}"
install -d -m 0755 "${systemd_dir}"

install -m 0755 "${build_dir}/biz-relay" "${prefix}/bin/biz-relay"
install -m 0755 "${build_dir}/biz-agent" "${prefix}/bin/biz-agent"
install -m 0755 "${build_dir}/biz-tunnelctl" "${prefix}/bin/biz-tunnelctl"

install -m 0644 packaging/systemd/biz-relay.service "${systemd_dir}/biz-relay.service"
install -m 0644 packaging/systemd/biz-agent.service "${systemd_dir}/biz-agent.service"

if [[ ! -f "${config_dir}/token" ]]; then
  "${prefix}/bin/biz-tunnelctl" gen-token --out "${config_dir}/token" >/dev/null
  chown biz-tunnel:biz-tunnel "${config_dir}/token"
  chmod 0640 "${config_dir}/token"
fi

systemctl daemon-reload

cat <<EOF
Installed biz-tunnel.

Next steps:
  1. Copy examples/relay.toml or examples/agent.toml into ${config_dir}/
  2. Replace inline tunnel.token with security.token_file = "${config_dir}/token" if desired
  3. Run: systemctl enable --now biz-relay.service
     or: systemctl enable --now biz-agent.service
EOF
