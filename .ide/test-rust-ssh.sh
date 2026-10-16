#!/bin/sh
# Run inside the CNB dev image as root. Uses an isolated loopback-only sshd;
# does not modify/reload the active SSH service or download dependencies.
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
config=${1:-"$script_dir/10-flamepanel-rust.conf"}
port=${FLAMEPANEL_TEST_SSH_PORT:-39222}
case "$port" in
    ''|*[!0-9]*) echo 'Invalid FLAMEPANEL_TEST_SSH_PORT' >&2; exit 1 ;;
esac
if [ "$port" -lt 1024 ] || [ "$port" -gt 65535 ]; then
    echo 'Test port must be between 1024 and 65535' >&2
    exit 1
fi
if [ "$(id -u)" -ne 0 ]; then
    echo 'Run this check as root inside the CNB development container.' >&2
    exit 1
fi
config=$(realpath -- "$config")
for tool in ssh ssh-keygen timeout; do
    command -v "$tool" >/dev/null
done
[ -x /usr/sbin/sshd ]
[ -r "$config" ]
# sshd requires its privilege-separation directory even with UsePAM=no.
[ -d /run/sshd ] || { echo '/run/sshd is missing; initialize the dev image first.' >&2; exit 1; }

umask 077
tmp=$(mktemp -d /tmp/flamepanel-rust-ssh.XXXXXX)
sshd_pid=
cleanup() {
    if [ -n "$sshd_pid" ]; then
        kill "$sshd_pid" 2>/dev/null || :
        wait "$sshd_pid" 2>/dev/null || :
    fi
    rm -rf -- "$tmp"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

ssh-keygen -q -t ed25519 -N '' -f "$tmp/host_key"
ssh-keygen -q -t ed25519 -N '' -f "$tmp/client_key"
cp "$tmp/client_key.pub" "$tmp/authorized_keys"
cat > "$tmp/sshd_config" <<EOF
Include "$config"
Port $port
ListenAddress 127.0.0.1
HostKey $tmp/host_key
PidFile $tmp/sshd.pid
AuthorizedKeysFile $tmp/authorized_keys
# The temporary authorized_keys lives outside /root; this exception applies
# only to this disposable test daemon, never to the production dev sshd.
StrictModes no
UsePAM no
PermitRootLogin prohibit-password
PubkeyAuthentication yes
PasswordAuthentication no
KbdInteractiveAuthentication no
AuthenticationMethods publickey
AllowUsers root
DisableForwarding yes
PermitTTY no
LogLevel ERROR
EOF

/usr/sbin/sshd -t -f "$tmp/sshd_config"
/usr/sbin/sshd -T -f "$tmp/sshd_config" > "$tmp/effective"
grep -Eq '^setenv .*RUSTUP_HOME=/usr/local/rustup([[:space:]]|$)' "$tmp/effective"
grep -Eq '^setenv .*CARGO_HOME=/usr/local/cargo([[:space:]]|$)' "$tmp/effective"
printf '[127.0.0.1]:%s ' "$port" > "$tmp/known_hosts"
awk '{print $1, $2}' "$tmp/host_key.pub" >> "$tmp/known_hosts"

# No inherited Rust homes on either side; the fix must come from sshd SetEnv.
env -u CARGO_HOME -u RUSTUP_HOME /usr/sbin/sshd -D -e -f "$tmp/sshd_config" > "$tmp/sshd.log" 2>&1 &
sshd_pid=$!
if env -u CARGO_HOME -u RUSTUP_HOME timeout 90 ssh -F /dev/null \
    -p "$port" -i "$tmp/client_key" \
    -o BatchMode=yes -o IdentitiesOnly=yes -o IdentityAgent=none \
    -o StrictHostKeyChecking=yes -o UserKnownHostsFile="$tmp/known_hosts" \
    -o GlobalKnownHostsFile=/dev/null -o ConnectionAttempts=5 -o ConnectTimeout=2 \
    root@127.0.0.1 'sh -s' <<'REMOTE'
set -eu
[ "${RUSTUP_HOME:-}" = /usr/local/rustup ]
[ "${CARGO_HOME:-}" = /usr/local/cargo ]
rustc --version
cargo --version
cargo fmt --version
cargo clippy --version
rustup show active-toolchain

smoke=$(mktemp -d /tmp/flamepanel-rust-smoke.XXXXXX)
trap 'rm -rf -- "$smoke"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
mkdir "$smoke/src"
cat > "$smoke/Cargo.toml" <<'TOML'
[package]
name = "flamepanel-ssh-smoke"
version = "0.0.0"
edition = "2021"
[workspace]
TOML
cat > "$smoke/src/main.rs" <<'RS'
fn main() {
    println!("rust-ssh-ok");
}

#[test]
fn toolchain_smoke() {
    assert_eq!(std::env::var("RUSTUP_HOME").unwrap(), "/usr/local/rustup");
    assert_eq!(std::env::var("CARGO_HOME").unwrap(), "/usr/local/cargo");
}
RS
cargo fmt --manifest-path "$smoke/Cargo.toml" -- --check
cargo build --offline --manifest-path "$smoke/Cargo.toml" --target-dir "$smoke/target"
cargo test --offline --manifest-path "$smoke/Cargo.toml" --target-dir "$smoke/target"
cargo clippy --offline --manifest-path "$smoke/Cargo.toml" --target-dir "$smoke/target" -- -D warnings
[ "$("$smoke/target/debug/flamepanel-ssh-smoke")" = rust-ssh-ok ]
echo 'PASS: non-interactive SSH Rust environment and offline smoke build/test/clippy'
REMOTE
then
    :
else
    result=$?
    cat "$tmp/sshd.log" >&2
    exit "$result"
fi
