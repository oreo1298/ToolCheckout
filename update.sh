#!/usr/bin/env bash
# ToolCheckout - update to the latest version (git pull, rebuild, reinstall the same way as before).
# Same options as install.sh, e.g.  ./update.sh --yes
exec "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/install.sh" --update "$@"
