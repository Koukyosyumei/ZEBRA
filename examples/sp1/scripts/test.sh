cargo build --release --example addsub
cargo build --release --example bitwise
cargo build --release --example branch
cargo build --release --example cpu
cargo build --release --example divrem
cargo build --release --example jump
cargo build --release --example lt
cargo build --release --example memoryreadwrite
cargo build --release --example mul
cargo build --release --example shiftleft
cargo build --release --example sr

./target/release/examples/addsub -- --config ./configs/alu.config --opcode-str "SUB" --method "bb" --num-trial 1