rm -f leaf.db*


RUST_BACKTRACE=1 RUSTFLAGS="-Awarnings" RUST_LOG=error cargo run -- db migrate up;

RUSTFLAGS="-Awarnings" RUST_LOG=error cargo run -- connections add --name=demo_source --username=system --password=Welcome1 --connection-string="localhost:1521/XEPDB1";

RUSTFLAGS="-Awarnings" RUST_LOG=error cargo run -- connections add --name=demo_target --username=system --password=Welcome1 --connection-string="localhost:1522/XEPDB1";

RUSTFLAGS="-Awarnings" RUST_LOG=error cargo run -- plans add --name demo1 --source demo_source --target demo_target --schemas "LEAF";

RUSTFLAGS="-Awarnings" RUST_LOG=error cargo run -- plans add --name demo2 --source demo_source --target demo_target --schemas "SCHEMA1,SCHEMA2";

RUSTFLAGS="-Awarnings" RUST_LOG=error cargo run -- plans add --name demo3 --source demo_source --target demo_target --schemas "SCHEMA3,SCHEMA4";
