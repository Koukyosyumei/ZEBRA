cargo build --release --example addsub
cargo build --release --example bitwise
cargo build --release --example branch
cargo build --release --example cloclz
cargo build --release --example divrem
cargo build --release --example jump
cargo build --release --example lt
cargo build --release --example memoryreadwrite
cargo build --release --example movcond
cargo build --release --example mul
cargo build --release --example shiftleft

echo "addsub"
./target/release/examples/addsub --config ./configs/alu.config --opcode-str "ADD" --method "bb" --num-trial 1
./target/release/examples/addsub --config ./configs/alu.config --opcode-str "SUB" --method "bb" --num-trial 1

echo "bitwise"
./target/release/examples/bitwise --config ./configs/alu.config --opcode-str "AND" --method "bb" --num-trial 1
./target/release/examples/bitwise --config ./configs/alu.config --opcode-str "OR" --method "bb" --num-trial 1
./target/release/examples/bitwise --config ./configs/alu.config --opcode-str "XOR" --method "bb" --num-trial 1

echo "cloclz"
./target/release/examples/cloclz --config ./configs/alu.config --opcode-str "CLO" --method "bb" --num-trial 1
./target/release/examples/cloclz --config ./configs/alu.config --opcode-str "CLZ" --method "bb" --num-trial 1

echo "branch"
./target/release/examples/branch --config ./configs/alu.config --opcode-str "BEQ" --method "bb" --num-trial 1
./target/release/examples/branch --config ./configs/alu.config --opcode-str "BGEZ" --method "bb" --num-trial 1
./target/release/examples/branch --config ./configs/alu.config --opcode-str "BGTZ" --method "bb" --num-trial 1
./target/release/examples/branch --config ./configs/alu.config --opcode-str "BLEZ" --method "bb" --num-trial 1
./target/release/examples/branch --config ./configs/alu.config --opcode-str "BLTZ" --method "bb" --num-trial 1
./target/release/examples/branch --config ./configs/alu.config --opcode-str "BNE" --method "bb" --num-trial 1

echo "divrem"
./target/release/examples/divrem --config ./configs/alu.config --opcode-str "DIV" --method "bb" --num-trial 1
./target/release/examples/divrem --config ./configs/alu.config --opcode-str "DIVU" --method "bb" --num-trial 1
./target/release/examples/divrem --config ./configs/alu.config --opcode-str "MOD" --method "bb" --num-trial 1
./target/release/examples/divrem --config ./configs/alu.config --opcode-str "MODU" --method "bb" --num-trial 1

echo "jump"
./target/release/examples/jump --config ./configs/alu.config --opcode-str "Jump" --method "bb" --num-trial 1
./target/release/examples/jump --config ./configs/alu.config --opcode-str "Jumpi" --method "bb" --num-trial 1
./target/release/examples/jump --config ./configs/alu.config --opcode-str "JumpDirect" --method "bb" --num-trial 1

echo "lt"
./target/release/examples/lt --config ./configs/alu.config --opcode-str "SLT" --method "bb" --num-trial 1
./target/release/examples/lt --config ./configs/alu.config --opcode-str "SLTU" --method "bb" --num-trial 1

echo "memoryreadwrite"
./target/release/examples/memoryreadwrite --config ./configs/alu.config --opcode-str "LB" --method "bb" --num-trial 1
./target/release/examples/memoryreadwrite --config ./configs/alu.config --opcode-str "LBU" --method "bb" --num-trial 1
./target/release/examples/memoryreadwrite --config ./configs/alu.config --opcode-str "LH" --method "bb" --num-trial 1
./target/release/examples/memoryreadwrite --config ./configs/alu.config --opcode-str "LHU" --method "bb" --num-trial 1
./target/release/examples/memoryreadwrite --config ./configs/alu.config --opcode-str "LW" --method "bb" --num-trial 1
./target/release/examples/memoryreadwrite --config ./configs/alu.config --opcode-str "SB" --method "bb" --num-trial 1
./target/release/examples/memoryreadwrite --config ./configs/alu.config --opcode-str "SH" --method "bb" --num-trial 1
./target/release/examples/memoryreadwrite --config ./configs/alu.config --opcode-str "SW" --method "bb" --num-trial 1
./target/release/examples/memoryreadwrite --config ./configs/alu.config --opcode-str "SC" --method "bb" --num-trial 1
./target/release/examples/memoryreadwrite --config ./configs/alu.config --opcode-str "SWL" --method "bb" --num-trial 1
./target/release/examples/memoryreadwrite --config ./configs/alu.config --opcode-str "SWR" --method "bb" --num-trial 1

echo "movcond"
./target/release/examples/movcond --config ./configs/alu.config --opcode-str "MEQ" --method "bb" --num-trial 1
./target/release/examples/movcond --config ./configs/alu.config --opcode-str "MNE" --method "bb" --num-trial 1
./target/release/examples/movcond --config ./configs/alu.config --opcode-str "WSBH" --method "bb" --num-trial 1

echo "mul"
./target/release/examples/mul --config ./configs/alu.config --opcode-str "MUL" --method "bb" --num-trial 1
./target/release/examples/mul --config ./configs/alu.config --opcode-str "MULT" --method "bb" --num-trial 1
./target/release/examples/mul --config ./configs/alu.config --opcode-str "MULTU" --method "bb" --num-trial 1

echo "sll"
./target/release/examples/shiftleft --config ./configs/alu.config --method "bb" --num-trial 1