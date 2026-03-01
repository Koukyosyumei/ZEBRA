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

echo "addsub"
./target/release/examples/addsub --config ./configs/alu.config --opcode-str "ADD" --method "bb" --num-trial 1
./target/release/examples/addsub --config ./configs/alu.config --opcode-str "SUB" --method "bb" --num-trial 1

echo "bitwise"
./target/release/examples/bitwise --config ./configs/alu.config --opcode-str "AND" --method "bb" --num-trial 1
./target/release/examples/bitwise --config ./configs/alu.config --opcode-str "OR" --method "bb" --num-trial 1
./target/release/examples/bitwise --config ./configs/alu.config --opcode-str "XOR" --method "bb" --num-trial 1

echo "branch"
./target/release/examples/branch --config ./configs/alu.config --opcode-str "BEQ" --method "bb" --num-trial 1
./target/release/examples/branch --config ./configs/alu.config --opcode-str "BGE" --method "bb" --num-trial 1
./target/release/examples/branch --config ./configs/alu.config --opcode-str "BLT" --method "bb" --num-trial 1
./target/release/examples/branch --config ./configs/alu.config --opcode-str "BNE" --method "bb" --num-trial 1

echo "divrem"
./target/release/examples/divrem --config ./configs/alu.config --opcode-str "DIV" --method "bb" --num-trial 1
./target/release/examples/divrem --config ./configs/alu.config --opcode-str "DIVU" --method "bb" --num-trial 1
./target/release/examples/divrem --config ./configs/alu.config --opcode-str "REM" --method "bb" --num-trial 1
./target/release/examples/divrem --config ./configs/alu.config --opcode-str "REMU" --method "bb" --num-trial 1

echo "jump"
./target/release/examples/jump --config ./configs/alu.config --opcode-str "JAL" --method "bb" --num-trial 1
./target/release/examples/jump --config ./configs/alu.config --opcode-str "JALR" --method "bb" --num-trial 1

echo "lt"
./target/release/examples/lt --config ./configs/alu.config --opcode-str "SLT" --method "bb" --num-trial 1
./target/release/examples/lt --config ./configs/alu.config --opcode-str "SLTU" --method "bb" --num-trial 1

echo "memoryreadwrite"
./target/release/examples/memoryreadwrite --config ./configs/alu.config --opcode-str "LB" --method "bb" --num-trial 1
./target/release/examples/memoryreadwrite --config ./configs/alu.config --opcode-str "LH" --method "bb" --num-trial 1
./target/release/examples/memoryreadwrite --config ./configs/alu.config --opcode-str "LW" --method "bb" --num-trial 1
./target/release/examples/memoryreadwrite --config ./configs/alu.config --opcode-str "LBU" --method "bb" --num-trial 1
./target/release/examples/memoryreadwrite --config ./configs/alu.config --opcode-str "LHU" --method "bb" --num-trial 1
./target/release/examples/memoryreadwrite --config ./configs/alu.config --opcode-str "SB" --method "bb" --num-trial 1
./target/release/examples/memoryreadwrite --config ./configs/alu.config --opcode-str "SH" --method "bb" --num-trial 1
./target/release/examples/memoryreadwrite --config ./configs/alu.config --opcode-str "SW" --method "bb" --num-trial 1

echo "mul"
./target/release/examples/mul --config ./configs/alu.config --opcode-str "MUL" --method "bb" --num-trial 1
./target/release/examples/mul --config ./configs/alu.config --opcode-str "MULH" --method "bb" --num-trial 1
./target/release/examples/mul --config ./configs/alu.config --opcode-str "MULHU" --method "bb" --num-trial 1
./target/release/examples/mul --config ./configs/alu.config --opcode-str "MULHSU" --method "bb" --num-trial 1

echo "sll"
./target/release/examples/shiftleft --config ./configs/alu.config --opcode-str "ADD" --method "bb" --num-trial 1

echo "sr"
./target/release/examples/sr --config ./configs/alu.config --opcode-str "SRL" --method "bb" --num-trial 1
./target/release/examples/sr --config ./configs/alu.config --opcode-str "SRA" --method "bb" --num-trial 1