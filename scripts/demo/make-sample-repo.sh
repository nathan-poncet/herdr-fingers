#!/usr/bin/env bash
# Builds the small git repository the demo is recorded in.
set -euo pipefail
target="${1:?usage: make-sample-repo.sh <directory>}"
rm -rf "$target"
mkdir -p "$target/src/api" "$target/docs"
cd "$target"
git init -q -b main .
git config user.name "Demo"
git config user.email "demo@example.com"
git config commit.gpgsign false

cat > README.md <<'MD'
# sample-app

Tiny service used for the herdr-fingers demo.

- Docs: https://docs.example.com/sample-app/getting-started
- Issues: https://github.com/nathan-poncet/herdr-fingers/issues
MD
printf 'fn main() {\n    println!("listening on 10.0.12.34:8080");\n}\n' > src/main.rs
printf 'pub fn routes() -> Vec<&'"'"'static str> {\n    vec!["/health", "/v1/orders"]\n}\n' > src/api/router.rs
printf '# Deploy\n\nRollout id: 550e8400-e29b-41d4-a716-446655440000\n' > docs/deploy.md
git add -A && git commit -q -m "Initial service skeleton"
printf '\nfn health() -> &'"'"'static str { "ok" }\n' >> src/main.rs
git commit -q -am "Add the health endpoint"
printf 'pub const VERSION: &str = "1.4.2";\n' > src/version.rs
git add src/version.rs && git commit -q -m "Expose the crate version"
printf '\n## Staging\n\nhttps://staging.example.com/sample-app\n' >> docs/deploy.md
printf '\n    vec!["/health", "/v1/orders", "/v1/invoices"]\n' >> src/api/router.rs
echo "sample repo ready in $target"
